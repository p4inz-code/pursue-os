//! Navigation request and result data models.
//!
//! Provides [`NavigationRequest`] and [`NavigationResult`] for dispatching and recording
//! HTTP/HTTPS web resource retrievals.
//!
//! # Secret Scrubbing Invariant
//! Web traffic may carry cookies, authentication headers, or sensitive session tokens.
//! The helper [`scrub_sensitive_headers`] ensures that sensitive authentication credentials
//! are never persisted into public provenance records or case manifests.

use std::collections::BTreeMap;

use pursue_core::{Error, Result};
use serde::{Deserialize, Serialize};

use crate::routing::RoutingMode;
use crate::url::ValidatedUrl;

/// Default navigation timeout in seconds (30 seconds).
pub const DEFAULT_NAV_TIMEOUT_SECS: u64 = 30;

/// Maximum permitted navigation timeout in seconds (5 minutes).
pub const MAX_NAV_TIMEOUT_SECS: u64 = 300;

/// Default maximum response payload buffer size (16 MiB).
pub const DEFAULT_MAX_RESPONSE_BYTES: usize = 16 * 1024 * 1024;

/// Absolute maximum response payload buffer size (64 MiB).
pub const MAX_ALLOWED_RESPONSE_BYTES: usize = 64 * 1024 * 1024;

/// A validated request to navigate to a web resource.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NavigationRequest {
    url: ValidatedUrl,
    timeout_secs: Option<u64>,
    max_response_bytes: Option<usize>,
    headers: BTreeMap<String, String>,
}

impl NavigationRequest {
    /// Creates a new navigation request for a validated URL.
    pub fn new(url: ValidatedUrl) -> Self {
        Self {
            url,
            timeout_secs: None,
            max_response_bytes: None,
            headers: BTreeMap::new(),
        }
    }

    /// Sets an execution timeout override.
    pub fn with_timeout(mut self, timeout_secs: u64) -> Result<Self> {
        if timeout_secs == 0 {
            return Err(Error::InvalidInput(
                "timeout must be greater than zero".into(),
            ));
        }
        if timeout_secs > MAX_NAV_TIMEOUT_SECS {
            return Err(Error::InvalidInput(format!(
                "timeout ({timeout_secs}s) exceeds maximum permitted ({MAX_NAV_TIMEOUT_SECS}s)"
            )));
        }
        self.timeout_secs = Some(timeout_secs);
        Ok(self)
    }

    /// Sets a maximum response body size limit in bytes.
    pub fn with_max_response_bytes(mut self, limit: usize) -> Result<Self> {
        if limit == 0 {
            return Err(Error::InvalidInput(
                "max_response_bytes must be greater than zero".into(),
            ));
        }
        if limit > MAX_ALLOWED_RESPONSE_BYTES {
            return Err(Error::InvalidInput(format!(
                "max_response_bytes ({limit}) exceeds maximum permitted ({MAX_ALLOWED_RESPONSE_BYTES})"
            )));
        }
        self.max_response_bytes = Some(limit);
        Ok(self)
    }

    /// Appends a request header.
    pub fn with_header(mut self, key: &str, value: &str) -> Result<Self> {
        let key = key.trim();
        if key.is_empty() {
            return Err(Error::InvalidInput("header key must not be empty".into()));
        }
        self.headers
            .insert(key.to_ascii_lowercase(), value.trim().to_string());
        Ok(self)
    }

    /// Returns the target URL.
    pub fn url(&self) -> &ValidatedUrl {
        &self.url
    }

    /// Returns the configured timeout, or the default.
    pub fn effective_timeout_secs(&self) -> u64 {
        self.timeout_secs.unwrap_or(DEFAULT_NAV_TIMEOUT_SECS)
    }

    /// Returns the configured maximum response bytes, or the default.
    pub fn effective_max_response_bytes(&self) -> usize {
        self.max_response_bytes
            .unwrap_or(DEFAULT_MAX_RESPONSE_BYTES)
    }

    /// Returns the request headers map.
    pub fn headers(&self) -> &BTreeMap<String, String> {
        &self.headers
    }
}

/// The outcome of navigating to a web resource.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NavigationResult {
    request_url: ValidatedUrl,
    final_url: ValidatedUrl,
    status_code: u16,
    headers: BTreeMap<String, String>,
    body: Vec<u8>,
    duration_ms: u64,
    routing_mode: RoutingMode,
    truncated: bool,
    timed_out: bool,
}

impl NavigationResult {
    /// Constructs a new [`NavigationResult`].
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        request_url: ValidatedUrl,
        final_url: ValidatedUrl,
        status_code: u16,
        headers: BTreeMap<String, String>,
        body: Vec<u8>,
        duration_ms: u64,
        routing_mode: RoutingMode,
        truncated: bool,
        timed_out: bool,
    ) -> Self {
        Self {
            request_url,
            final_url,
            status_code,
            headers,
            body,
            duration_ms,
            routing_mode,
            truncated,
            timed_out,
        }
    }

    /// Returns the originally requested URL.
    pub fn request_url(&self) -> &ValidatedUrl {
        &self.request_url
    }

    /// Returns the final URL reached (after any redirects).
    pub fn final_url(&self) -> &ValidatedUrl {
        &self.final_url
    }

    /// Returns the HTTP status code (e.g. 200, 404).
    pub fn status_code(&self) -> u16 {
        self.status_code
    }

    /// Returns `true` if the status code indicates HTTP success (200..=299).
    pub fn is_success(&self) -> bool {
        (200..=299).contains(&self.status_code)
    }

    /// Returns the response headers.
    pub fn headers(&self) -> &BTreeMap<String, String> {
        &self.headers
    }

    /// Returns the response body bytes.
    pub fn body(&self) -> &[u8] {
        &self.body
    }

    /// Returns the request roundtrip duration in milliseconds.
    pub fn duration_ms(&self) -> u64 {
        self.duration_ms
    }

    /// Returns the network routing mode employed for this request.
    pub fn routing_mode(&self) -> RoutingMode {
        self.routing_mode
    }

    /// Returns `true` if the response body exceeded the buffer limit and was truncated.
    pub fn is_truncated(&self) -> bool {
        self.truncated
    }

    /// Returns `true` if the request timed out.
    pub fn is_timed_out(&self) -> bool {
        self.timed_out
    }

    /// Returns a copy of the response headers with sensitive headers scrubbed.
    pub fn safe_headers(&self) -> BTreeMap<String, String> {
        scrub_sensitive_headers(&self.headers)
    }
}

/// Scrubs sensitive headers from a header map.
///
/// Removes or redacts authorization tokens, cookies, session identifiers,
/// and API keys to prevent secret leakage in logs and provenance records.
pub fn scrub_sensitive_headers(headers: &BTreeMap<String, String>) -> BTreeMap<String, String> {
    const SENSITIVE_KEYS: &[&str] = &[
        "authorization",
        "cookie",
        "set-cookie",
        "proxy-authorization",
        "x-api-key",
        "api-key",
        "bearer",
        "session-token",
    ];

    let mut clean = BTreeMap::new();
    for (k, v) in headers {
        let lower = k.to_ascii_lowercase();
        if SENSITIVE_KEYS.iter().any(|&s| lower == s) {
            clean.insert(k.clone(), "[REDACTED]".to_string());
        } else {
            clean.insert(k.clone(), v.clone());
        }
    }
    clean
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_construction_and_validation() {
        let url = ValidatedUrl::parse("https://example.com/test").unwrap();
        let req = NavigationRequest::new(url.clone())
            .with_timeout(10)
            .unwrap()
            .with_max_response_bytes(1024 * 1024)
            .unwrap()
            .with_header("User-Agent", "PursueBrowser/1.0")
            .unwrap();

        assert_eq!(req.url(), &url);
        assert_eq!(req.effective_timeout_secs(), 10);
        assert_eq!(req.effective_max_response_bytes(), 1024 * 1024);
        assert_eq!(
            req.headers().get("user-agent").unwrap(),
            "PursueBrowser/1.0"
        );
    }

    #[test]
    fn request_rejects_invalid_limits() {
        let url = ValidatedUrl::parse("https://example.com/test").unwrap();
        assert!(NavigationRequest::new(url.clone()).with_timeout(0).is_err());
        assert!(
            NavigationRequest::new(url.clone())
                .with_timeout(301)
                .is_err()
        );
        assert!(
            NavigationRequest::new(url.clone())
                .with_max_response_bytes(0)
                .is_err()
        );
        assert!(
            NavigationRequest::new(url.clone())
                .with_max_response_bytes(65 * 1024 * 1024)
                .is_err()
        );
    }

    #[test]
    fn secret_scrubbing_redacts_auth_and_cookie_headers() {
        let mut headers = BTreeMap::new();
        headers.insert("Content-Type".into(), "text/html".into());
        headers.insert("Authorization".into(), "Bearer secret-token-12345".into());
        headers.insert("Cookie".into(), "session=abcde".into());

        let scrubbed = scrub_sensitive_headers(&headers);
        assert_eq!(scrubbed.get("Content-Type").unwrap(), "text/html");
        assert_eq!(scrubbed.get("Authorization").unwrap(), "[REDACTED]");
        assert_eq!(scrubbed.get("Cookie").unwrap(), "[REDACTED]");
    }
}
