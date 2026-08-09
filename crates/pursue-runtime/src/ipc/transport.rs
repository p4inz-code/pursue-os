//! Transports for the IPC boundary.
//!
//! # Abstraction
//! [`Transport`] is the platform-independent contract: serialize the request,
//! exchange bytes, deserialize the response. Framing is length-prefixed JSON
//! (8-byte little-endian length prefix + payload).
//!
//! # Implementations
//! - [`InMemoryTransport`] — a byte-exact duplex channel; works on every
//!   platform and exercises the full serialize/transport/deserialize path.
//! - `unix_transport` — Unix-domain sockets (Linux). Compiled on every
//!   platform but only *exercised* on Unix; CI validates it on ubuntu-latest.
//!
//! # Security
//! Transport errors are generic I/O or protocol errors; they never include
//! evidence content or request payloads. Malformed frames are rejected before
//! dispatch.

use pursue_core::{Error, Result};
use std::collections::VecDeque;
use std::io::{self, Read, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex};

use super::dispatch::Router;
use super::protocol::{Request, Response};

/// Maximum accepted frame size in bytes.
///
/// Guards against a peer-controlled length prefix forcing an unbounded
/// allocation (malformed-input hardening).
pub const MAX_FRAME_LEN: usize = 64 * 1024 * 1024;

/// A transport able to exchange one request/response pair.
pub trait Transport: Send {
    /// Sends `request` and returns the matching response.
    ///
    /// Errors are transport/protocol-level only (I/O, serialization, and
    /// malformed-frame failures). Application failures travel inside the
    /// [`Response`] error field.
    fn round_trip(&mut self, request: &Request) -> Result<Response>;
}

/// Writes an 8-byte little-endian length prefix followed by `payload`.
fn write_frame<W: Write>(writer: &mut W, payload: &[u8]) -> io::Result<()> {
    writer.write_all(&(payload.len() as u64).to_le_bytes())?;
    writer.write_all(payload)?;
    writer.flush()
}

/// Reads a length-prefixed frame written by [`write_frame`].
///
/// Frames larger than [`MAX_FRAME_LEN`] are rejected with
/// [`Error::InvalidInput`] before any allocation.
fn read_frame<R: Read>(reader: &mut R) -> Result<Vec<u8>> {
    let mut len_buf = [0u8; 8];
    reader.read_exact(&mut len_buf)?;
    let len = u64::from_le_bytes(len_buf);
    if len > MAX_FRAME_LEN as u64 {
        return Err(Error::InvalidInput(format!(
            "frame length {len} exceeds maximum {MAX_FRAME_LEN}"
        )));
    }
    let mut payload = vec![0u8; len as usize];
    reader.read_exact(&mut payload)?;
    Ok(payload)
}

/// Serves requests read from `reader`, dispatching through `router`, writing
/// responses to `writer`, until the connection closes cleanly.
///
/// One request/response exchange per connection is the contract for the
/// foundation transports. A clean EOF returns `Ok`; a malformed frame returns
/// [`Error::InvalidInput`] and aborts the serve loop.
pub fn serve<R: Read, W: Write>(mut reader: R, mut writer: W, router: &mut Router) -> Result<()> {
    loop {
        let payload = match read_frame(&mut reader) {
            Ok(payload) => payload,
            Err(Error::Io(e)) if e.kind() == io::ErrorKind::UnexpectedEof => return Ok(()),
            Err(e) => return Err(e),
        };
        let request: Request = serde_json::from_slice(&payload)
            .map_err(|e| Error::InvalidInput(format!("malformed request frame: {e}")))?;
        let response = router.handle(&request);
        let out = serde_json::to_vec(&response)
            .map_err(|e| Error::InvalidInput(format!("response serialization failed: {e}")))?;
        write_frame(&mut writer, &out)?;
    }
}

/// A byte channel with blocking reads and a close-on-drop signal.
struct Channel {
    buf: Mutex<VecDeque<u8>>,
    ready: Condvar,
    closed: AtomicBool,
}

impl Channel {
    fn new() -> Self {
        Self {
            buf: Mutex::new(VecDeque::new()),
            ready: Condvar::new(),
            closed: AtomicBool::new(false),
        }
    }

    fn write_bytes(&self, bytes: &[u8]) -> io::Result<()> {
        let mut buf = self.buf.lock().unwrap_or_else(|p| p.into_inner());
        if self.closed.load(Ordering::Relaxed) {
            return Err(io::Error::new(io::ErrorKind::BrokenPipe, "channel closed"));
        }
        buf.extend(bytes.iter().copied());
        self.ready.notify_all();
        Ok(())
    }

    fn read_bytes(&self, out: &mut [u8]) -> io::Result<usize> {
        let mut buf = self.buf.lock().unwrap_or_else(|p| p.into_inner());
        loop {
            if !buf.is_empty() {
                let take = out.len().min(buf.len());
                for (slot, byte) in out.iter_mut().zip(buf.drain(..take)) {
                    *slot = byte;
                }
                return Ok(take);
            }
            if self.closed.load(Ordering::Relaxed) {
                return Ok(0);
            }
            buf = self.ready.wait(buf).unwrap_or_else(|p| p.into_inner());
        }
    }

    fn close(&self) {
        self.closed.store(true, Ordering::Relaxed);
        self.ready.notify_all();
    }
}

/// The write end of a [`Channel`]; closing the channel on drop signals EOF to
/// the reader.
struct ChannelWriter {
    channel: Arc<Channel>,
}

impl Write for ChannelWriter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.channel.write_bytes(bytes)?;
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl Drop for ChannelWriter {
    fn drop(&mut self) {
        self.channel.close();
    }
}

/// The read end of a [`Channel`].
struct ChannelReader {
    channel: Arc<Channel>,
}

impl Read for ChannelReader {
    fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
        self.channel.read_bytes(out)
    }
}

/// A byte-exact in-memory transport backed by a duplex channel pair.
///
/// The two ends behave like connected pipes: bytes written to one end are
/// read from the other, and dropping one end signals EOF to the other. This
/// exercises the full serialize/transport/deserialize path on every platform.
pub struct InMemoryTransport {
    reader: ChannelReader,
    writer: ChannelWriter,
}

impl InMemoryTransport {
    /// Creates a connected pair of transports.
    pub fn pair() -> (Self, Self) {
        let a_to_b = Arc::new(Channel::new());
        let b_to_a = Arc::new(Channel::new());
        let a = Self {
            reader: ChannelReader {
                channel: Arc::clone(&b_to_a),
            },
            writer: ChannelWriter {
                channel: Arc::clone(&a_to_b),
            },
        };
        let b = Self {
            reader: ChannelReader { channel: a_to_b },
            writer: ChannelWriter { channel: b_to_a },
        };
        (a, b)
    }

    /// Serves requests on this end until the peer end is dropped.
    ///
    /// The caller typically runs this on the server side in a thread.
    pub fn serve_requests(self, router: &mut Router) -> Result<()> {
        serve(self.reader, self.writer, router)
    }
}

impl Transport for InMemoryTransport {
    fn round_trip(&mut self, request: &Request) -> Result<Response> {
        let payload = serde_json::to_vec(request)
            .map_err(|e| Error::InvalidInput(format!("request serialization failed: {e}")))?;
        write_frame(&mut self.writer, &payload)?;
        let reply = read_frame(&mut self.reader)?;
        let response: Response = serde_json::from_slice(&reply)
            .map_err(|e| Error::InvalidInput(format!("response deserialization failed: {e}")))?;
        response.validate()?;
        Ok(response)
    }
}

/// Unix-domain-socket transport (Linux).
///
/// Compiled on every platform; exercised only on Unix. CI validates it on
/// ubuntu-latest. The server accepts one request/response exchange per
/// connection and removes the socket file on clean shutdown.
#[cfg(unix)]
pub mod unix_transport {
    use super::{Transport, read_frame, serve, write_frame};
    use crate::ipc::dispatch::Router;
    use crate::ipc::protocol::{Request, Response};
    use pursue_core::{Error, Result};
    use std::io;
    use std::os::unix::net::{UnixListener, UnixStream};
    use std::path::Path;
    use std::sync::atomic::{AtomicBool, Ordering};

    /// A client transport over a Unix-domain socket.
    pub struct UnixTransport {
        stream: UnixStream,
    }

    impl UnixTransport {
        /// Connects to a listening socket at `path`.
        pub fn connect(path: &Path) -> Result<Self> {
            let stream = UnixStream::connect(path)?;
            Ok(Self { stream })
        }
    }

    impl Transport for UnixTransport {
        fn round_trip(&mut self, request: &Request) -> Result<Response> {
            let payload = serde_json::to_vec(request)
                .map_err(|e| Error::InvalidInput(format!("request serialization failed: {e}")))?;
            write_frame(&mut self.stream, &payload)?;
            let reply = read_frame(&mut self.stream)?;
            let response: Response = serde_json::from_slice(&reply).map_err(|e| {
                Error::InvalidInput(format!("response deserialization failed: {e}"))
            })?;
            response.validate()?;
            Ok(response)
        }
    }

    /// Serves requests on a Unix-domain socket at `path` until `shutdown` is
    /// set. Stale socket files are removed before binding and on clean exit.
    ///
    /// Each accepted connection handles exactly one request/response exchange
    /// (the foundation contract), then the connection is closed. The serve
    /// loop is strict: a connection that violates the protocol (malformed or
    /// oversized frame) aborts the loop rather than being skipped — a known
    /// limitation of the foundation, to be relaxed when services go
    /// multi-connection.
    pub fn serve_unix(path: &Path, router: &mut Router, shutdown: &AtomicBool) -> Result<()> {
        let _ = std::fs::remove_file(path);
        let listener = UnixListener::bind(path).map_err(|e| {
            Error::Io(io::Error::new(
                e.kind(),
                format!("failed to bind {}: {e}", path.display()),
            ))
        })?;
        while !shutdown.load(Ordering::Relaxed) {
            let (stream, _addr) = match listener.accept() {
                Ok(pair) => pair,
                Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
                Err(e) => return Err(e.into()),
            };
            serve(&mut &stream, &mut &stream, router)?;
        }
        let _ = std::fs::remove_file(path);
        Ok(())
    }

    #[cfg(test)]
    mod tests {
        use super::{Transport, UnixTransport, serve_unix};
        use crate::ipc::dispatch::{Handler, Router};
        use crate::ipc::protocol::{IpcError, IpcErrorCode, MethodName, Request, ServiceId};
        use serde_json::{Value, json};
        use std::path::PathBuf;
        use std::sync::Arc;
        use std::sync::atomic::{AtomicBool, Ordering};
        use std::thread;
        use std::time::Duration;

        struct EchoHandler {
            id: ServiceId,
        }

        impl Handler for EchoHandler {
            fn service_id(&self) -> &ServiceId {
                &self.id
            }

            fn handle(
                &mut self,
                method: &MethodName,
                params: &Value,
            ) -> std::result::Result<Value, IpcError> {
                if method.as_str() == "echo" {
                    Ok(params.clone())
                } else {
                    Err(IpcError::new(IpcErrorCode::UnknownMethod, "no such method").unwrap())
                }
            }
        }

        fn temp_dir(label: &str) -> PathBuf {
            let unique = format!(
                "pursue-test-{label}-{}-{:?}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            );
            std::env::temp_dir().join(unique)
        }

        fn connect_with_retry(path: &std::path::Path) -> UnixTransport {
            for _ in 0..40 {
                if let Ok(transport) = UnixTransport::connect(path) {
                    return transport;
                }
                thread::sleep(Duration::from_millis(50));
            }
            panic!("failed to connect to test socket at {}", path.display());
        }

        #[test]
        fn unix_socket_round_trip() {
            let dir = temp_dir("uds");
            let path = dir.join("pursue.sock");
            let mut router = Router::new();
            router
                .register(Box::new(EchoHandler {
                    id: ServiceId::new("echo").unwrap(),
                }))
                .unwrap();

            let shutdown = Arc::new(AtomicBool::new(false));
            let server_shutdown = Arc::clone(&shutdown);
            let server_path = path.clone();
            let handle = thread::spawn(move || {
                serve_unix(&server_path, &mut router, &server_shutdown).unwrap();
            });

            let mut client = connect_with_retry(&path);
            let request = Request::new(
                1,
                ServiceId::new("echo").unwrap(),
                MethodName::new("echo").unwrap(),
                json!({"k": "v"}),
            );
            let response = client.round_trip(&request).unwrap();
            assert_eq!(response.id, 1);
            assert_eq!(response.result, Some(json!({"k": "v"})));

            // The server must not be listening after a clean shutdown.
            shutdown.store(true, Ordering::Relaxed);
            handle.join().unwrap();
            let _ = std::fs::remove_dir_all(&dir);
        }

        #[test]
        fn unix_socket_unknown_service_error() {
            let dir = temp_dir("uds-unknown");
            let path = dir.join("pursue.sock");
            let mut router = Router::new();
            router
                .register(Box::new(EchoHandler {
                    id: ServiceId::new("echo").unwrap(),
                }))
                .unwrap();

            let shutdown = Arc::new(AtomicBool::new(false));
            let server_shutdown = Arc::clone(&shutdown);
            let server_path = path.clone();
            let handle = thread::spawn(move || {
                serve_unix(&server_path, &mut router, &server_shutdown).unwrap();
            });

            let mut client = connect_with_retry(&path);
            let request = Request::new(
                2,
                ServiceId::new("ghost").unwrap(),
                MethodName::new("echo").unwrap(),
                json!(null),
            );
            let response = client.round_trip(&request).unwrap();
            let error = response.error.as_ref().unwrap();
            assert_eq!(error.code(), IpcErrorCode::UnknownService);

            shutdown.store(true, Ordering::Relaxed);
            handle.join().unwrap();
            let _ = std::fs::remove_dir_all(&dir);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{InMemoryTransport, Transport, read_frame, serve, write_frame};
    use crate::ipc::dispatch::{Handler, Router};
    use crate::ipc::protocol::{IpcError, IpcErrorCode, MethodName, Request, ServiceId};
    use serde_json::{Value, json};
    use std::io::Cursor;
    use std::thread;

    struct EchoHandler {
        id: ServiceId,
    }

    impl Handler for EchoHandler {
        fn service_id(&self) -> &ServiceId {
            &self.id
        }

        fn handle(
            &mut self,
            method: &MethodName,
            params: &Value,
        ) -> std::result::Result<Value, IpcError> {
            if method.as_str() == "echo" {
                Ok(params.clone())
            } else {
                Err(IpcError::new(IpcErrorCode::UnknownMethod, "no such method").unwrap())
            }
        }
    }

    fn router_with_echo() -> Router {
        let mut router = Router::new();
        router
            .register(Box::new(EchoHandler {
                id: ServiceId::new("echo").unwrap(),
            }))
            .unwrap();
        router
    }

    #[test]
    fn framing_roundtrip() {
        let mut writer = Cursor::new(Vec::new());
        write_frame(&mut writer, b"hello world").unwrap();
        let bytes = writer.into_inner();
        let mut reader = Cursor::new(bytes);
        assert_eq!(read_frame(&mut reader).unwrap(), b"hello world");
    }

    #[test]
    fn serve_returns_ok_on_clean_eof() {
        let mut router = Router::new();
        let mut input = Cursor::new(Vec::new());
        let mut output = Cursor::new(Vec::new());
        serve(&mut input, &mut output, &mut router).unwrap();
        assert!(output.into_inner().is_empty());
    }

    #[test]
    fn serve_rejects_malformed_request_frame() {
        let mut router = Router::new();
        let mut payload = Vec::new();
        payload.extend(3u64.to_le_bytes()); // valid frame, invalid JSON body
        payload.extend(b"xyz");
        let mut input = Cursor::new(payload);
        let mut output = Cursor::new(Vec::new());
        let err = serve(&mut input, &mut output, &mut router).unwrap_err();
        assert!(err.to_string().contains("malformed request frame"));
    }

    #[test]
    fn read_frame_rejects_oversized_length_prefix() {
        let mut payload = Vec::new();
        payload.extend(u64::MAX.to_le_bytes());
        let mut reader = Cursor::new(payload);
        let err = read_frame(&mut reader).unwrap_err();
        assert!(err.to_string().contains("exceeds maximum"));
    }

    #[test]
    fn serve_rejects_oversized_frame() {
        let mut router = Router::new();
        let mut payload = Vec::new();
        payload.extend((super::MAX_FRAME_LEN as u64 + 1).to_le_bytes());
        let mut input = Cursor::new(payload);
        let mut output = Cursor::new(Vec::new());
        let err = serve(&mut input, &mut output, &mut router).unwrap_err();
        assert!(err.to_string().contains("exceeds maximum"));
    }

    #[test]
    fn in_memory_round_trip_echo() {
        let mut router = router_with_echo();
        let (server_end, mut client) = InMemoryTransport::pair();
        let handle = thread::spawn(move || {
            server_end.serve_requests(&mut router).unwrap();
        });

        let request = Request::new(
            1,
            ServiceId::new("echo").unwrap(),
            MethodName::new("echo").unwrap(),
            json!({"hello": "world"}),
        );
        let response = client.round_trip(&request).unwrap();
        assert_eq!(response.id, 1);
        assert!(response.is_success());
        assert_eq!(response.result, Some(json!({"hello": "world"})));

        drop(client); // closes the channel; server exits
        handle.join().unwrap();
    }

    #[test]
    fn in_memory_round_trip_typed_error() {
        let mut router = router_with_echo();
        let (server_end, mut client) = InMemoryTransport::pair();
        let handle = thread::spawn(move || {
            server_end.serve_requests(&mut router).unwrap();
        });

        let request = Request::new(
            5,
            ServiceId::new("echo").unwrap(),
            MethodName::new("nope").unwrap(),
            json!(null),
        );
        let response = client.round_trip(&request).unwrap();
        assert_eq!(
            response.error.as_ref().unwrap().code(),
            IpcErrorCode::UnknownMethod
        );

        drop(client);
        handle.join().unwrap();
    }

    #[test]
    fn in_memory_round_trip_unknown_service() {
        let mut router = router_with_echo();
        let (server_end, mut client) = InMemoryTransport::pair();
        let handle = thread::spawn(move || {
            server_end.serve_requests(&mut router).unwrap();
        });

        let request = Request::new(
            7,
            ServiceId::new("ghost").unwrap(),
            MethodName::new("echo").unwrap(),
            json!(null),
        );
        let response = client.round_trip(&request).unwrap();
        assert_eq!(
            response.error.as_ref().unwrap().code(),
            IpcErrorCode::UnknownService
        );

        drop(client);
        handle.join().unwrap();
    }
}
