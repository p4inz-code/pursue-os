//! Browser execution engines.
//!
//! Provides the [`BrowserEngine`] trait along with:
//! - [`MockBrowserEngine`]: Fully deterministic, programmable engine for unit and integration testing.
//! - [`NetworkEngine`]: Production HTTP/HTTPS network engine with direct TCP and Tor SOCKS5h routing.
//!
//! # Critical Network Boundaries
//! - If `session.mode() == RoutingMode::Tor` and Tor is down or unreachable, the engine
//!   **FAILS CLOSED**. It will never fall back to direct networking.
//! - If `session.mode() == RoutingMode::Direct`, requests are routed directly and will never
//!   silently route through Tor.
//! - Domains ending in `.onion` require `RoutingMode::Tor`; attempting to navigate them
//!   in direct mode is rejected.
//! - Remote DNS resolution (SOCKS5h) is strictly enforced when Tor is active to prevent
//!   local DNS leakage.

use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use pursue_core::{Error, Result};

use crate::navigation::{NavigationRequest, NavigationResult};
use crate::routing::{RoutingMode, TorConfig};
use crate::session::BrowserSession;
use crate::url::ValidatedUrl;

/// Trait abstracting web navigation and network dispatch.
pub trait BrowserEngine: Send + Sync {
    /// Dispatches a navigation request within the context of a browser session.
    fn navigate(
        &self,
        session: &BrowserSession,
        request: &NavigationRequest,
    ) -> Result<NavigationResult>;

    /// Returns `true` if the Tor routing subsystem is currently operational.
    fn is_tor_available(&self) -> bool;
}

/// A deterministic mock engine for automated testing without live network dependencies.
#[derive(Default)]
pub struct MockBrowserEngine {
    tor_available: Mutex<bool>,
    responses: Mutex<BTreeMap<String, MockResponse>>,
    default_response: Mutex<Option<MockResponse>>,
    recorded_calls: Mutex<Vec<(String, NavigationRequest, RoutingMode)>>,
}

/// A programmed outcome for the mock engine.
#[derive(Clone, Debug)]
pub enum MockResponse {
    /// Return an HTTP response.
    Success {
        /// HTTP status code.
        status_code: u16,
        /// Response headers.
        headers: BTreeMap<String, String>,
        /// Response body.
        body: Vec<u8>,
        /// Final redirected URL (if different).
        final_url: Option<ValidatedUrl>,
    },
    /// Simulate a network or service failure.
    Failure(String),
}

impl MockBrowserEngine {
    /// Creates a new mock browser engine with Tor available by default.
    pub fn new() -> Self {
        Self {
            tor_available: Mutex::new(true),
            responses: Mutex::new(BTreeMap::new()),
            default_response: Mutex::new(None),
            recorded_calls: Mutex::new(Vec::new()),
        }
    }

    /// Sets whether Tor is available for tests simulating Tor outages.
    pub fn set_tor_available(&self, available: bool) {
        *self.tor_available.lock().unwrap() = available;
    }

    /// Registers a mock response for a matching URL prefix.
    pub fn register(&self, url_prefix: &str, response: MockResponse) {
        self.responses
            .lock()
            .unwrap()
            .insert(url_prefix.to_string(), response);
    }

    /// Sets a default fallback response for unregistered URLs.
    pub fn set_default(&self, response: MockResponse) {
        *self.default_response.lock().unwrap() = Some(response);
    }

    /// Returns all recorded navigation calls: `(session_id, request, mode)`.
    pub fn recorded_calls(&self) -> Vec<(String, NavigationRequest, RoutingMode)> {
        self.recorded_calls.lock().unwrap().clone()
    }
}

impl BrowserEngine for MockBrowserEngine {
    fn navigate(
        &self,
        session: &BrowserSession,
        request: &NavigationRequest,
    ) -> Result<NavigationResult> {
        if !session.is_active() {
            return Err(Error::InvalidInput(format!(
                "cannot navigate in terminated session {}",
                session.id()
            )));
        }

        let mode = session.mode();

        // Enforce: .onion requires Tor
        if request.url().is_onion() && mode.is_direct() {
            return Err(Error::InvalidInput(format!(
                "cannot navigate .onion domain '{}' in Direct mode; Tor routing required",
                request.url().host()
            )));
        }

        // Enforce: Tor requested but unavailable -> FAIL CLOSED (no fallback to direct)
        if mode.is_tor() && !self.is_tor_available() {
            return Err(Error::ServiceFailure(
                "Tor proxy is unavailable; failing closed without fallback to direct networking"
                    .into(),
            ));
        }

        self.recorded_calls
            .lock()
            .unwrap()
            .push((session.id().to_string(), request.clone(), mode));

        let url_str = request.url().as_str();
        let outcome = {
            let map = self.responses.lock().unwrap();
            let mut found = None;
            for (prefix, resp) in map.iter() {
                if url_str.starts_with(prefix) {
                    found = Some(resp.clone());
                    break;
                }
            }
            found.or_else(|| self.default_response.lock().unwrap().clone())
        };

        match outcome {
            Some(MockResponse::Success {
                status_code,
                headers,
                body,
                final_url,
            }) => {
                let limit = request.effective_max_response_bytes();
                let truncated = body.len() > limit;
                let body_slice = if truncated {
                    body[..limit].to_vec()
                } else {
                    body
                };
                let effective_final = final_url.unwrap_or_else(|| request.url().clone());

                Ok(NavigationResult::new(
                    request.url().clone(),
                    effective_final,
                    status_code,
                    headers,
                    body_slice,
                    15,
                    mode,
                    truncated,
                    false,
                ))
            }
            Some(MockResponse::Failure(msg)) => Err(Error::ServiceFailure(msg)),
            None => {
                // Default echo response (200 OK with empty body)
                Ok(NavigationResult::new(
                    request.url().clone(),
                    request.url().clone(),
                    200,
                    BTreeMap::new(),
                    Vec::new(),
                    5,
                    mode,
                    false,
                    false,
                ))
            }
        }
    }

    fn is_tor_available(&self) -> bool {
        *self.tor_available.lock().unwrap()
    }
}

/// Production network engine performing direct TCP/HTTP and Tor SOCKS5h routing.
#[derive(Clone, Debug, Default)]
pub struct NetworkEngine {
    tor_config: TorConfig,
}

impl NetworkEngine {
    /// Creates a new network engine with default Tor configuration (`127.0.0.1:9050`).
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a network engine with customized Tor proxy settings.
    pub fn with_tor_config(tor_config: TorConfig) -> Self {
        Self { tor_config }
    }

    /// Connects via SOCKS5 proxy using domain name addressing (SOCKS5h) to prevent DNS leaks.
    fn connect_socks5(&self, host: &str, port: u16, timeout: Duration) -> Result<TcpStream> {
        let mut stream = TcpStream::connect_timeout(&self.tor_config.socks_proxy, timeout)
            .map_err(|e| {
                Error::ServiceFailure(format!(
                    "failed to connect to Tor SOCKS5 proxy at {}: {e}",
                    self.tor_config.socks_proxy
                ))
            })?;

        stream.set_read_timeout(Some(timeout)).map_err(Error::Io)?;
        stream.set_write_timeout(Some(timeout)).map_err(Error::Io)?;

        // Handshake: [Version: 5, Methods Count: 1, Method: 0 (No Auth)]
        stream.write_all(&[0x05, 0x01, 0x00]).map_err(Error::Io)?;
        let mut auth_resp = [0u8; 2];
        stream.read_exact(&mut auth_resp).map_err(Error::Io)?;
        if auth_resp[0] != 0x05 || auth_resp[1] != 0x00 {
            return Err(Error::ServiceFailure(format!(
                "Tor SOCKS5 authentication rejected: {:?}",
                auth_resp
            )));
        }

        // Request: [Version: 5, Cmd: 1 (CONNECT), Reserved: 0, AddrType: 3 (Domain Name)]
        let host_bytes = host.as_bytes();
        if host_bytes.len() > 255 {
            return Err(Error::InvalidInput(
                "hostname exceeds 255 bytes for SOCKS5".into(),
            ));
        }

        let mut req = Vec::with_capacity(4 + 1 + host_bytes.len() + 2);
        req.extend_from_slice(&[0x05, 0x01, 0x00, 0x03]);
        req.push(host_bytes.len() as u8);
        req.extend_from_slice(host_bytes);
        req.extend_from_slice(&port.to_be_bytes());

        stream.write_all(&req).map_err(Error::Io)?;

        // Response header: [Version, Rep, Reserved, AddrType]
        let mut header = [0u8; 4];
        stream.read_exact(&mut header).map_err(Error::Io)?;
        if header[1] != 0x00 {
            return Err(Error::ServiceFailure(format!(
                "Tor SOCKS5 proxy connect failed with reply code 0x{:02x}",
                header[1]
            )));
        }

        // Drain bound address based on AddrType
        match header[3] {
            0x01 => {
                // IPv4: 4 bytes + 2 bytes port
                let mut buf = [0u8; 6];
                stream.read_exact(&mut buf).map_err(Error::Io)?;
            }
            0x03 => {
                // Domain: 1 byte len + N bytes + 2 bytes port
                let mut len_buf = [0u8; 1];
                stream.read_exact(&mut len_buf).map_err(Error::Io)?;
                let mut rest = vec![0u8; len_buf[0] as usize + 2];
                stream.read_exact(&mut rest).map_err(Error::Io)?;
            }
            0x04 => {
                // IPv6: 16 bytes + 2 bytes port
                let mut buf = [0u8; 18];
                stream.read_exact(&mut buf).map_err(Error::Io)?;
            }
            other => {
                return Err(Error::ServiceFailure(format!(
                    "unrecognized SOCKS5 address type 0x{other:02x}"
                )));
            }
        }

        Ok(stream)
    }

    /// Performs direct TCP connection.
    fn connect_direct(&self, host: &str, port: u16, timeout: Duration) -> Result<TcpStream> {
        let addrs = format!("{host}:{port}").to_socket_addrs().map_err(|e| {
            Error::ServiceFailure(format!("DNS resolution failed for '{host}': {e}"))
        })?;

        let mut last_err = None;
        for addr in addrs {
            match TcpStream::connect_timeout(&addr, timeout) {
                Ok(stream) => {
                    let _ = stream.set_read_timeout(Some(timeout));
                    let _ = stream.set_write_timeout(Some(timeout));
                    return Ok(stream);
                }
                Err(e) => last_err = Some(e),
            }
        }

        Err(Error::ServiceFailure(format!(
            "failed to connect directly to {host}:{port}: {:?}",
            last_err
        )))
    }
}

impl BrowserEngine for NetworkEngine {
    fn navigate(
        &self,
        session: &BrowserSession,
        request: &NavigationRequest,
    ) -> Result<NavigationResult> {
        if !session.is_active() {
            return Err(Error::InvalidInput(format!(
                "cannot navigate in terminated session {}",
                session.id()
            )));
        }

        let mode = session.mode();

        // Enforce: .onion requires Tor
        if request.url().is_onion() && mode.is_direct() {
            return Err(Error::InvalidInput(format!(
                "cannot navigate .onion domain '{}' in Direct mode; Tor routing required",
                request.url().host()
            )));
        }

        let timeout = Duration::from_secs(request.effective_timeout_secs());
        let host = request.url().host();
        let port = request.url().port();

        let start = Instant::now();

        // Establish connection according to explicit routing mode (FAIL CLOSED)
        let mut stream = match mode {
            RoutingMode::Direct => self.connect_direct(host, port, timeout)?,
            RoutingMode::Tor => {
                // Strict fail-closed: if Tor connection fails, error out immediately
                self.connect_socks5(host, port, timeout)?
            }
        };

        // Format minimal raw HTTP/1.1 request
        let mut req_bytes = format!(
            "GET {} HTTP/1.1\r\nHost: {}\r\nUser-Agent: PURSUE-OS-Browser/0.1\r\nConnection: close\r\n",
            request.url().path_and_query(),
            host
        );
        for (k, v) in request.headers() {
            req_bytes.push_str(&format!("{k}: {v}\r\n"));
        }
        req_bytes.push_str("\r\n");

        stream.write_all(req_bytes.as_bytes()).map_err(Error::Io)?;

        // Read response up to max limit
        let limit = request.effective_max_response_bytes();
        let mut raw_response = Vec::new();
        let mut chunk = [0u8; 8192];
        let mut truncated = false;

        loop {
            match stream.read(&mut chunk) {
                Ok(0) => break,
                Ok(n) => {
                    let space = (limit + 65536).saturating_sub(raw_response.len());
                    if space >= n {
                        raw_response.extend_from_slice(&chunk[..n]);
                    } else {
                        if space > 0 {
                            raw_response.extend_from_slice(&chunk[..space]);
                        }
                        truncated = true;
                    }
                }
                Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(e)
                    if e.kind() == std::io::ErrorKind::WouldBlock
                        || e.kind() == std::io::ErrorKind::TimedOut =>
                {
                    return Ok(NavigationResult::new(
                        request.url().clone(),
                        request.url().clone(),
                        0,
                        BTreeMap::new(),
                        Vec::new(),
                        start.elapsed().as_millis() as u64,
                        mode,
                        false,
                        true,
                    ));
                }
                Err(e) => return Err(Error::Io(e)),
            }
        }

        let duration_ms = start.elapsed().as_millis() as u64;

        // Parse status line and headers
        let (status_code, headers, body) = parse_raw_http(&raw_response, limit);

        Ok(NavigationResult::new(
            request.url().clone(),
            request.url().clone(),
            status_code,
            headers,
            body,
            duration_ms,
            mode,
            truncated,
            false,
        ))
    }

    fn is_tor_available(&self) -> bool {
        // Quick probe to SOCKS5 port
        TcpStream::connect_timeout(&self.tor_config.socks_proxy, Duration::from_millis(500)).is_ok()
    }
}

/// Helper parsing simple raw HTTP/1.1 response bytes into (status, headers, body).
fn parse_raw_http(raw: &[u8], limit: usize) -> (u16, BTreeMap<String, String>, Vec<u8>) {
    let mut headers = BTreeMap::new();
    let delim = b"\r\n\r\n";

    let (head_part, body_part) = if let Some(idx) = raw.windows(4).position(|w| w == delim) {
        (&raw[..idx], &raw[idx + 4..])
    } else {
        (raw, &[][..])
    };

    let head_str = String::from_utf8_lossy(head_part);
    let mut lines = head_str.lines();

    let mut status_code = 0;
    if let Some(status_line) = lines.next() {
        let parts: Vec<&str> = status_line.split_whitespace().collect();
        if parts.len() >= 2 {
            if let Ok(code) = parts[1].parse::<u16>() {
                status_code = code;
            }
        }
    }

    for line in lines {
        if let Some((k, v)) = line.split_once(':') {
            headers.insert(k.trim().to_ascii_lowercase(), v.trim().to_string());
        }
    }

    let body = if body_part.len() > limit {
        body_part[..limit].to_vec()
    } else {
        body_part.to_vec()
    };

    (status_code, headers, body)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session_id::BrowserSessionId;
    use pursue_case::CaseId;
    use std::path::Path;

    fn create_test_session(mode: RoutingMode) -> BrowserSession {
        BrowserSession::new(
            BrowserSessionId::new("test-sess").unwrap(),
            CaseId::new("case-100").unwrap(),
            "investigator",
            mode,
            Path::new("/tmp"),
        )
        .unwrap()
    }

    #[test]
    fn mock_engine_routes_and_records() {
        let engine = MockBrowserEngine::new();
        let session = create_test_session(RoutingMode::Direct);
        let url = ValidatedUrl::parse("http://example.com/api").unwrap();
        let req = NavigationRequest::new(url.clone());

        let res = engine.navigate(&session, &req).unwrap();
        assert_eq!(res.status_code(), 200);
        assert_eq!(res.routing_mode(), RoutingMode::Direct);

        let calls = engine.recorded_calls();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].0, "test-sess");
        assert_eq!(calls[0].2, RoutingMode::Direct);
    }

    #[test]
    fn mock_engine_rejects_onion_in_direct_mode() {
        let engine = MockBrowserEngine::new();
        let session = create_test_session(RoutingMode::Direct);
        let url = ValidatedUrl::parse("http://testservice.onion/page").unwrap();
        let req = NavigationRequest::new(url);

        let err = engine.navigate(&session, &req).unwrap_err();
        assert!(err.to_string().contains("Tor routing required"));
    }

    #[test]
    fn mock_engine_tor_unavailable_fails_closed_without_direct_fallback() {
        let engine = MockBrowserEngine::new();
        engine.set_tor_available(false); // Simulate Tor down

        let session = create_test_session(RoutingMode::Tor);
        let url = ValidatedUrl::parse("http://example.com/secure").unwrap();
        let req = NavigationRequest::new(url);

        let err = engine.navigate(&session, &req).unwrap_err();
        assert!(
            err.to_string()
                .contains("Tor proxy is unavailable; failing closed")
        );

        // Verify zero requests were sent to the network or recorded
        assert_eq!(engine.recorded_calls().len(), 0);
    }

    #[test]
    fn mock_engine_truncates_large_payloads() {
        let engine = MockBrowserEngine::new();
        let big_body = vec![b'A'; 2000];
        engine.register(
            "http://example.com/big",
            MockResponse::Success {
                status_code: 200,
                headers: BTreeMap::new(),
                body: big_body,
                final_url: None,
            },
        );

        let session = create_test_session(RoutingMode::Direct);
        let url = ValidatedUrl::parse("http://example.com/big").unwrap();
        let req = NavigationRequest::new(url)
            .with_max_response_bytes(500)
            .unwrap();

        let res = engine.navigate(&session, &req).unwrap();
        assert_eq!(res.body().len(), 500);
        assert!(res.is_truncated());
    }
}
