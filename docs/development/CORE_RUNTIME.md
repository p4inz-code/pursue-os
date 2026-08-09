# PURSUE OS — Core Runtime (Phase 1C)

> **Category:** B — implementation documentation (maintained with the code).
> **Scope:** Phase 1C core runtime: configuration, structured logging, service
> lifecycle, IPC/service boundary. See `docs/core/DECISION_RECORD.md` Part B
> (rows B9–B12) for the decisions behind this design.

## Overview

`crates/pursue-runtime` provides the in-process runtime boundary that core
services build on. It is deliberately small: no daemon framework, no service
manager, no process supervision. systemd remains the eventual Linux
supervisor (see `docs/architecture/SERVICE_MODEL.md`); this crate is the Rust
application/service boundary inside a single process.

Modules:

| Module | Purpose |
|---|---|
| `log` | Structured logging: levels, records, sinks, components, redaction |
| `config` | Typed configuration: explicit defaults, TOML loading, validation |
| `service` | Minimal lifecycle: `init -> start -> run -> shutdown` |
| `ipc` | Typed request/response protocol, service dispatch, transports |

Dependencies: `pursue-core` (shared primitives), `serde`, `serde_json`,
`toml`. No new heavyweight dependencies. `unsafe` code remains denied
(workspace lint).

## Configuration architecture

- **Defaults vs. configuration are separate.** `Config::defaults()` is the
  explicit baseline (format version 1, `log_level = "info"`, no paths). A TOML
  document overlays only the keys it contains — every field carries
  `#[serde(default)]` — so partial user/system configuration is additive and
  deterministic.
- **Format:** TOML via the `toml` crate; output is deterministic
  pretty-printed TOML (`Config::to_toml_string`).
- **Validation:** `Config::validate` rejects unsupported format versions and
  present-but-empty paths; `Config::from_toml_str` parses, overlays defaults,
  and validates in one step.
- **Secrets:** configuration never contains secrets by design. Credentials,
  tokens, and keys must be supplied via environment or OS secret facilities
  in later phases; there is no secret field in the config model.
- `ipc_socket_path` is meaningful on Unix only; on Windows it is accepted but
  unused (see Platform limitations).

## Logging architecture

- **Levels:** `Error < Warn < Info < Debug < Trace`. A logger with threshold
  `L` emits records at `level` when `level <= L`; filtering happens before any
  sink is touched.
- **Records:** timestamped (Unix seconds + RFC 3339 UTC, formatted with a
  dependency-free civil-date algorithm), leveled, component-scoped, with
  optional structured key/value fields. Serialized as one JSON object per
  line.
- **Sinks:** pluggable via the `Sink` trait. Default is `StderrSink` (JSON
  lines on standard error, keeping stdout clean for data output). `TestSink`
  captures records in memory for tests. `NullSink` discards.
- **Logger:** `Logger` combines level filtering, component scoping, and a
  sink. `init_global` installs a process-wide instance (`global()` /
  `try_global()`); services can also construct their own loggers.
- **Sink errors are best-effort:** a failing sink is ignored so logging can
  never disrupt the process.
- **Security:** logs are diagnostics, **not** evidence. Services must never
  log raw evidence contents, secrets, credentials, private keys, or
  authentication tokens. `log::redact` wraps sensitive values so every output
  path renders `***`. Nothing in the logging module reads file contents,
  environment secrets, or evidence data.

## Runtime lifecycle

- A `Service` implements `init`, `start`, `run`, `shutdown`. `Runtime` drives
  one service (or a sequence via `run_all`) through the phases.
- **Guarantees:** once `init` succeeds, `shutdown` is always attempted, even
  if `start` or `run` fails. The first failing phase wins and its error
  (wrapped in `pursue_core::Error::ServiceFailure` naming the service and
  phase) is propagated to the caller.
- A shared shutdown flag lets long-running `run` implementations observe
  `Runtime::request_shutdown` via `ServiceContext::shutdown_requested`.
- This is not a service manager: no restart, monitoring, or process
  spawning. Linux supervision is systemd's role.

## IPC boundary

Layered, platform-independent-first:

- **Protocol** (`ipc::protocol`): typed serde types — `Request`
  (`id`, `service`, `method`, `params`), `Response` (exactly one of `result`
  or `error`), `ServiceId`/`MethodName` (validated identifiers, also validated
  on deserialization so malformed wire names are rejected before dispatch),
  and `IpcError`/`IpcErrorCode` (stable machine-readable codes: invalid
  request, unknown service/method, invalid params, permission denied,
  internal, service unavailable).
- **Dispatch** (`ipc::dispatch`): `Handler` per service; `Router` routes
  requests by service identity and rejects duplicate registrations. Unknown
  services receive a typed `unknown_service` response.
- **Transport** (`ipc::transport`): the `Transport` trait is the
  platform-independent contract. Framing is length-prefixed JSON (8-byte
  little-endian length prefix + payload).
  - `InMemoryTransport` — byte-exact duplex channel; works on every platform
    and exercises the full serialize/transport/deserialize path.
  - `unix_transport::UnixTransport` / `serve_unix` — Unix-domain sockets.
    Compiled on every platform; **only exercised on Unix** (CI validates on
    ubuntu-latest). One request/response exchange per connection; the socket
    file is removed on clean shutdown.
- **Security:** protocol and transport errors are diagnostics only and never
  include evidence content, secrets, or payloads. Malformed frames are
  rejected before dispatch, and frames exceeding `MAX_FRAME_LEN` (64 MiB) are
  rejected before allocation. Authorizing privileged operations is a
  *service-layer* concern; the boundary provides the typed channel, not the
  authority decision. No listeners are created implicitly.

## Platform limitations

- Development currently happens on Windows; Unix-domain-socket support is
  compiled everywhere but validated only where the platform supports it
  (locally on Linux hosts and by CI on ubuntu-latest). Windows validation
  covers the protocol, dispatch, and the in-memory transport round-trip.
- `Config::ipc_socket_path` is a Unix concept; Windows ignores it.
- Nothing Linux-specific is claimed as validated on Windows. `cargo check
  --target x86_64-unknown-linux-gnu` is used to typecheck the Unix transport
  from the Windows dev host.

## Security considerations

- `unsafe_code = "deny"` throughout (workspace lint).
- Logging: no evidence content; `redact` for secrets; best-effort sinks.
- Configuration: no secret fields; version gate and path validation.
- IPC: validated identifiers, typed errors without sensitive data, malformed
  frames rejected pre-dispatch, transports opt-in.
- These foundations are not a claim of hardening; privilege separation,
  sandboxing, and storage encryption remain later-phase concerns
  (see `docs/architecture/SECURITY_BOUNDARIES.md`).

## Validation

```bash
cd "/c/Users/Admin/Desktop/Pursue-OS" && cargo fmt --all --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && cargo build --workspace --release
```

The Unix transport is typechecked from Windows with:

```bash
cargo check --workspace --target x86_64-unknown-linux-gnu
```
