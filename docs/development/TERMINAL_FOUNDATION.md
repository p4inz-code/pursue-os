# PURSUE OS — Terminal Foundation (Phase 1E Specification)

> **Category:** B — Implementation documentation (maintained with the code).  
> **Status:** Fully implemented and verified in Phase 1E (221/221 workspace tests passing, clippy clean, Linux cross-compile clean).  
> **Scope:** Phase 1E Investigation Terminal Foundation: session management, command execution abstraction, terminal evidence capture, provenance attribution, runtime service integration, and IPC boundary.

---

## 1. Purpose

The **Investigation Terminal** is one of the two flagship interfaces of PURSUE OS (the other being the Investigation Browser; see `docs/core/MASTER_SPEC.md` §4 and `docs/core/PURSUE_V1_CONTRACT.md` §2, §5).

### Why It Exists
Investigative work (OSINT, SOCMINT, GEOINT, CTI, and DFIR) fundamentally relies on command-line utilities, web scrapers, automated scripts, network scanners, and framework tools. In conventional systems, investigator terminal actions are ephemeral: command outputs are lost when the terminal buffer rolls over, copying output into notes is manual and error-prone, and provenance (who ran what command, when, with what arguments, yielding what exact output) is lost.

PURSUE OS elevates the terminal to a purpose-built investigation instrument where:
- Executed commands, exit codes, environments, and outputs can be captured directly into the active case.
- Captured artifacts are cryptographically hashed and stored in the tamper-evident evidence foundation (`pursue-evidence`).
- The relationship between investigator actions and collected evidence is recorded in immutable, append-only audit chains (`pursue-case`).
- Security boundaries are explicit: execution is investigator-controlled, environment secrets are protected from leakage, and output sizes are strictly bounded.

### What Phase 1E Delivers
Phase 1E builds the **Investigation Terminal Foundation** in a new crate: `crates/pursue-terminal`.  
It delivers the headless core engine:
1. Validated session and command data models.
2. A trait-based command execution abstraction (`CommandExecutor`) supporting deterministic mock execution and OS process execution.
3. Automated evidence packaging that transforms command output into immutable `EvidenceRecord` items and attaches them to `Case` containers.
4. Terminal service lifecycle and IPC dispatch handlers integrating with `pursue-runtime`.
5. Strict resource and security boundaries (bounded outputs, timeout enforcement, secret scrubbing).

---

## 2. Phase 1E Goals

- **G1:** Provide a clean, robust data model for terminal sessions, command invocations, execution metadata, and captured output streams.
- **G2:** Provide a trait-based execution abstraction allowing unit tests to run deterministically with mock executors and real process execution to run safely on host platforms.
- **G3:** Implement evidence capture workflows: turn command stdout, stderr, or combined outputs into content-addressed evidence stored via `EvidenceStore` and attached to `Case` via `CaseStore`.
- **G4:** Ensure all state-changing terminal and evidence actions append events to the case's hash-chained `AuditLog`.
- **G5:** Integrate as a managed `pursue-runtime::service::Service` exposed across the `pursue-runtime::ipc` boundary.
- **G6:** Maintain strict memory safety (`#![deny(unsafe_code)]`), thorough documentation (`#![warn(missing_docs)]`), and 100% offline, deterministic automated tests.

---

## 3. Explicit Non-Goals (Out of Scope for Phase 1E)

To prevent premature complexity and maintain architectural integrity, Phase 1E explicitly excludes:
- **No full GUI terminal window:** No graphical desktop window, GPU rendering, or terminal emulator frontend (e.g., Alacritty/VTE widget). Phase 1E is the headless engine/service.
- **No interactive PTY implementation in V1 core:** No raw termios/pty terminal master/slave multiplexing yet. Batch execution is implemented first; the execution trait leaves an explicit expansion path for PTYs.
- **No Tor or browser integration:** Tor routing, proxying, and browser integration belong strictly to Phase 1F (`docs/core/MASTER_SPEC.md` §15).
- **No online AI APIs:** External AI APIs are a hard V1 non-goal (`docs/core/V1_SCOPE.md`).
- **No local AI execution engine:** Model weight loading and local inference (Ollama/llama.cpp) are separate platform services added in later phases.
- **No broad OSINT tool collection:** No packaging or bundling of third-party OSINT scripts; tools will be integrated once the execution boundary is secure.
- **No plugin marketplace:** Third-party plugin management and remote distribution are deferred to V2 (`docs/core/V1_SCOPE.md`).
- **No database backends:** SQLite remains deferred per `DECISION_RECORD.md` B4/B13; trait abstractions allow adding it later without rework.

---

## 4. Crate Boundary (`crates/pursue-terminal`)

A new workspace crate `crates/pursue-terminal` will be created under `crates/`.

### Responsibilities (Inside Crate)
- Terminal session lifecycle management (`SessionId`, `Session`, `SessionStatus`).
- Command execution requests, execution contexts, and execution records (`CommandRequest`, `ExecutionResult`, `StreamOutput`).
- Trait definition for command execution (`CommandExecutor`), with an in-memory/mock implementation (`MockExecutor`) and a local process implementation (`ProcessExecutor`).
- Evidence capture orchestration: extracting raw output streams, computing metadata, constructing canonical evidence records, persisting blobs into the case's `FileStore`, and updating the `Case`.
- Terminal service implementation (`TerminalService`) conforming to `pursue-runtime::service::Service`.
- Terminal IPC dispatch handler (`TerminalHandler`) conforming to `pursue-runtime::ipc::dispatch::Handler`.
- Resource policing: output size limits, timeout guards, and environment variable redaction.

### Boundaries (Outside Crate)
- Cryptographic hashing and blob storage -> belongs to `pursue-evidence`.
- Case manifest persistence, audit log chaining, and case isolation -> belongs to `pursue-case`.
- Service lifecycle supervision, log sinks, and IPC transports -> belongs to `pursue-runtime`.
- Desktop UI rendering and terminal styling -> belongs to future UI crates (`ui/terminal/`).
- System supervisor and daemon launching -> belongs to Linux `systemd`.

---

## 5. Integration Architecture

The terminal crate sits atop the existing foundation crates in a strict directed acyclic dependency graph:

```
crates/pursue-terminal
    ├── depends on ──> crates/pursue-runtime   (Config, Logger, Service, IPC Router)
    ├── depends on ──> crates/pursue-case      (Case, CaseId, CaseStore)
    ├── depends on ──> crates/pursue-evidence  (ContentAddress, EvidenceRecord, EvidenceStore)
    └── depends on ──> crates/pursue-core      (Error, Result, hex)
```

### Ownership and Data Flow
1. **Session Creation:** Investigator or IPC client requests a session linked to an active `CaseId`. `TerminalService` validates that the case exists via `CaseStore::contains_case`.
2. **Command Dispatch:** Client submits a `CommandRequest` (binary name, arguments, environment overrides, working directory).
3. **Execution:** `CommandExecutor` executes the command, enforcing timeouts and output buffer size limits, returning an `ExecutionResult`.
4. **Evidence Ingestion (Investigator-Triggered or Auto-Policy):**
   - The raw byte streams (`stdout`, `stderr`, or combined `ExecutionResult`) are passed to the case's evidence store via `FileCaseStore::open_evidence_store(case_id)`.
   - `EvidenceStore::put(&bytes, source_label, actor)` stores the immutable blob under `cases/<case_id>/evidence/blobs/<hex>.bin` and records an `acquired` audit event.
   - The resulting `ContentAddress` is attached to the `Case` via `Case::attach(&address, actor)`.
   - `FileCaseStore::save_case(&case)` writes the updated `case.json` manifest with recomputed `case_hash`.
5. **Diagnostics:** Execution telemetry is logged via `pursue-runtime::log::Logger` with sensitive variables scrubbed via `pursue_runtime::log::redact`.

---

## 6. Terminal Session & Command Model

### Types & Entities

1. **`SessionId`:**
   - Validated ASCII identifier (`[A-Za-z0-9._-]`, length 1..=64), safe for diagnostics and IPC routing.
2. **`Session`:**
   - Container for an ongoing investigation terminal session.
   - Fields:
     - `id: SessionId`
     - `case_id: CaseId` (the anchored investigation container)
     - `actor: String` (the authenticated investigator)
     - `created_at_unix: u64`
     - `working_dir: PathBuf` (enforced working directory)
     - `env_vars: BTreeMap<String, String>` (sanitized session environment)
     - `status: SessionStatus` (`Active`, `Terminated`)
3. **`CommandRequest`:**
   - Represents an explicit invocation request.
   - Fields:
     - `program: String` (the executable name or path; checked against traversal rules)
     - `args: Vec<String>` (discrete argument list; never raw shell strings)
     - `working_dir: Option<PathBuf>` (overrides session working dir if within allowed boundaries)
     - `env_overrides: BTreeMap<String, String>`
     - `capture_as_evidence: bool` (whether output should automatically be ingested into the case)
     - `source_label: Option<String>` (label describing the source if captured)
4. **`ExecutionResult`:**
   - Captures the concrete outcome of execution.
   - Fields:
     - `command_id: String` (unique invocation ID)
     - `exit_code: Option<i32>` (`None` if terminated by signal/timeout)
     - `stdout: Vec<u8>`
     - `stderr: Vec<u8>`
     - `started_at_unix: u64`
     - `finished_at_unix: u64`
     - `timed_out: bool`
     - `output_truncated: bool`
     - `evidence_address: Option<ContentAddress>` (present if stored as evidence)

---

## 7. Command Execution Abstraction

### The `CommandExecutor` Trait

To guarantee testability and architectural separation, execution is abstracted behind a trait:

```rust
pub trait CommandExecutor: Send + Sync {
    /// Executes a command request within the context of a session, returning
    /// the bounded execution result.
    fn execute(
        &self,
        session: &Session,
        request: &CommandRequest,
    ) -> Result<ExecutionResult>;
}
```

### Implementations

1. **`MockExecutor`:**
   - In-memory, deterministic executor for unit and integration testing.
   - Holds pre-programmed responses or simulated scripts based on program name.
   - Validates that error conditions (timeouts, non-zero exits, truncated outputs) behave identically across Windows and Linux.
2. **`ProcessExecutor`:**
   - Executes real OS processes using `std::process::Command`.
   - **No shell wrapping by default:** Invokes the executable directly via `execve`-style argument passing, eliminating shell-injection risks.
   - Captures `stdout` and `stderr` up to `MAX_OUTPUT_BYTES`.
   - Implements synchronous timeout enforcement with process termination on expiration.
3. **Future Extension (`PtyExecutor`):**
   - Designed to slot behind `CommandExecutor` or a companion `InteractiveExecutor` trait when interactive PTY support is added, without breaking existing batch command contracts.

---

## 8. Evidence Capture Workflow

Evidence capture turns volatile command execution output into immutable forensic records.

### Stream Capture Policy
- **Separation of Streams:** Raw `stdout` and `stderr` are captured as distinct byte vectors.
- **Evidentiary Blob Options:**
  - *Standard Mode:* Raw `stdout` bytes form the primary evidence artifact. If `stderr` contains diagnostics, it is either stored as a companion record or captured in an investigator-selected combined transcript.
  - *Transcript Mode:* A deterministic canonical JSON structure encoding command metadata, exit code, stdout, and stderr can be serialized and content-addressed as a structured execution artifact.
- **Deterministic Storage:**
  - The byte slice is hashed via `ContentAddress::hash(&bytes)`.
  - Stored in the case's evidence repository via `EvidenceStore::put(data, source_label, actor)`.
  - Attached to the `Case` by storing only the 32-byte `ContentAddress`.
  - Appends `case.evidence.attached` to the case's `AuditLog`.
  - **Zero Duplication:** Evidence payload bytes are stored once under `cases/<case_id>/evidence/blobs/<hex>.bin`; the `Case` object references only the SHA-256 address.

---

## 9. Provenance Model

Terminal operations must be attributable without making unjustified claims of absolute real-world verification.

### Recorded Provenance Tuple
Whenever evidence is generated from the terminal, the audit record links:
1. `timestamp_unix`: Wall-clock UTC timestamp when execution began.
2. `actor`: The investigator identifier responsible for triggering the command.
3. `action`: The explicit audit action (`case.evidence.attached`).
4. `subject`: The SHA-256 `ContentAddress` of the captured output.
5. `metadata`: In the evidence record's source string, a structured attribution tag: `terminal://session/<session_id>/cmd/<program>?exit=<exit_code>`.

### Non-Repudiation Boundary
- The cryptographic hash chain guarantees that *within the PURSUE OS case file*, the recorded command, exit status, and evidence association have not been modified post-recording.
- It does **not** claim or pretend that the remote web server or external process output was free from deception before it entered the terminal.

---

## 10. ANSI & Terminal Control Handling

In interactive shells, processes emit ANSI escape sequences (`\x1b[31m`, cursor movements, clear screens).

### Phase 1E Rules
1. **Raw Output as Source of Truth:**
   - When output is captured as evidence, the **raw byte stream** as produced by the process is stored. Stripping ANSI codes before hashing would alter the original bytes, violating the principle that evidence is the immutable source of truth.
2. **Display Sanitization (Non-Evidentiary):**
   - For terminal log display or IPC inspection, a utility helper `strip_ansi(bytes: &[u8]) -> String` is provided to allow safe text viewing without terminal injection attacks.
3. **No Terminal Emulator / Renderer in Core:**
   - Phase 1E does not include an ANSI terminal state machine emulator. Screen grid rendering is left to the future UI layer.

---

## 11. Security Model & Resource Limits

Command execution is inherently powerful; the terminal foundation enforces explicit security guardrails.

### 1. Argument & Execution Safety
- Executables are called directly via array-based arguments (`argv`). Shell expansion (`sh -c` or `cmd.exe /c`) is strictly forbidden unless the investigator explicitly requests shell evaluation.
- Prevents command injection and argument splitting vulnerabilities.

### 2. Working Directory & Path Traversal
- The session working directory must be an existing, canonicalized path.
- Attempts to pass relative path traversal (`../../etc/shadow`) are rejected during request validation.

### 3. Environment Sanitization & Secret Scrubbing
- Child processes do not inherit arbitrary dirty environments by default.
- A configurable denylist scrubs sensitive environment variables (e.g., `AWS_SECRET_ACCESS_KEY`, `SSH_AUTH_SOCK`, `PURSUE_API_KEY`) from execution records and logs.
- Environment variables in diagnostics are wrapped using `pursue_runtime::log::redact`.

### 4. Resource Bounding
- **`MAX_OUTPUT_BYTES`:** Default 16 MiB per execution stream (`16 * 1024 * 1024` bytes). Output exceeding this is truncated, setting `output_truncated: true` on `ExecutionResult`. This prevents process memory exhaustion.
- **`MAX_COMMAND_TIMEOUT`:** Default 60 seconds for batch execution. If a process does not exit within the timeout, it is killed (`SIGKILL` on Unix, `TerminateProcess` on Windows), and `timed_out: true` is returned.
- **`MAX_SESSIONS`:** Maximum active concurrent sessions bounded to prevent descriptor leakage.

### 5. Case Isolation
- A terminal session is bound to exactly one `CaseId`.
- Captured evidence is persisted strictly to that case's directory structure (`cases/<case_id>/evidence/`).
- The session cannot attach evidence to a different case without explicit session re-anchoring.

---

## 12. IPC & Runtime Service Integration

`pursue-terminal` integrates cleanly into `pursue-runtime`:

### Service Lifecycle
`TerminalService` implements `pursue_runtime::service::Service`:
- `init`: Validates configuration (`Config`), confirms base data directory.
- `start`: Initializes session tracking table and execution worker thread pool.
- `run`: Listens for shutdown requests on `ServiceContext::shutdown_requested()`.
- `shutdown`: Terminates any running child processes gracefully, closes active sessions, and flushes pending evidence writes.

### IPC Protocol Contract
`TerminalHandler` implements `pursue_runtime::ipc::dispatch::Handler`:
- `service_id`: `ServiceId::new("terminal")`
- Supported Methods:
  1. `session.create` -> params: `{ "case_id": "case-1", "working_dir": "/path" }` -> returns `{ "session_id": "..." }`
  2. `session.close` -> params: `{ "session_id": "..." }` -> returns `{ "closed": true }`
  3. `command.execute` -> params: `CommandRequest` -> returns `ExecutionResult`
  4. `evidence.capture` -> params: `{ "session_id": "...", "command_id": "...", "source_label": "..." }` -> returns `{ "content_address": "..." }`

---

## 13. Error Model

The terminal foundation reuses `pursue_core::Error` and defines domain-specific variants where necessary:

- `Error::InvalidInput`: Malformed session ID, invalid arguments, non-existent working directory.
- `Error::NotFound`: Unknown session ID, unknown case ID, missing output stream.
- `Error::IntegrityViolation`: Evidence verification failure upon attaching output.
- `Error::ServiceFailure`: Subprocess execution failure, timeout expiration, or failure to kill timed-out child process.

Terminal-specific error conversions map into typed `pursue_runtime::ipc::protocol::IpcError` with stable codes (`InvalidParams`, `NotFound`, `Internal`).

---

## 14. Testing Strategy

Following `docs/development/TESTING_STRATEGY.md`, Phase 1E requires comprehensive test coverage:

### Test Suites
1. **Unit Tests (`crates/pursue-terminal/src/`):**
   - `session_id_validation`: Valid characters, length boundaries (1..=64), whitespace rejection.
   - `session_lifecycle`: Creation, status transitions (`Active` -> `Terminated`), idempotent close.
   - `command_request_validation`: Empty program name rejection, path traversal rejection.
   - `mock_executor_behavior`: Verifies exit codes, stdout/stderr capture, timeout flag, truncation flag.
   - `evidence_capture_formatting`: Deterministic hashing of output bytes, `EvidenceRecord` structure.
   - `secret_scrubbing`: Ensures denylisted environment variables never appear in logs or execution records.
2. **Process Execution Tests:**
   - Real process execution using standard cross-platform utilities (`echo` or standard test executables).
   - Verifies exit code capture (`0` for success, non-zero for failure).
   - Verifies timeout termination behavior on long-running processes (`sleep`).
   - Verifies buffer truncation when output exceeds `MAX_OUTPUT_BYTES`.
3. **Integration Tests (`crates/pursue-terminal/tests/`):**
   - End-to-end flow: Create session -> Execute command -> Capture output as evidence -> Verify evidence blob exists in `FileCaseStore` -> Verify address attached to `Case` -> Verify `AuditLog` verifies cleanly.
   - Case isolation test: Ensure session bound to `Case A` cannot persist evidence into `Case B`.
4. **IPC Dispatch Tests:**
   - Dispatch `command.execute` and `session.create` over `pursue_runtime::ipc::dispatch::Router`.

---

## 15. Platform Strategy

### Development Environment (Windows Dev Host)
- All data models, session management, mock executors, evidence capture logic, and IPC dispatch run and test identically on Windows.
- Real process execution tests use standard cross-platform commands or Rust test harness binaries.

### Target Environment (Linux)
- Linux-specific process handling (signal termination via `SIGKILL`, termios flags if introduced) is gated with `#[cfg(unix)]`.
- `cargo check --workspace --target x86_64-unknown-linux-gnu` ensures strict compile-time verification from Windows.
- GitHub Actions CI on `ubuntu-latest` exercises live process execution on real Linux kernels.

---

## 16. Future PTY Compatibility

While Phase 1E deliberately avoids building an interactive PTY emulator, the design ensures future compatibility:
- `CommandRequest` and `ExecutionResult` cleanly separate raw streams from interactive state.
- The `CommandExecutor` trait accepts a `Session` and `CommandRequest`; a future `PtyExecutor` can implement a companion `InteractiveSession` without altering how batch command evidence is stored in `pursue-evidence` and `pursue-case`.

---

## 17. Resource Limits (Defensible Defaults)

| Parameter | Value | Rationale |
|---|---|---|
| `SESSION_ID_MAX_LEN` | 64 bytes | Consistent with `MAX_NAME_LEN` in `pursue-runtime::ipc`. |
| `DEFAULT_OUTPUT_LIMIT` | 16 MiB (`16,777,216` bytes) | Generous for text output; prevents OOM on runaway commands. |
| `DEFAULT_TIMEOUT_SECS` | 60 seconds | Prevents orphaned processes from hanging batch runs indefinitely. |
| `MAX_ACTIVE_SESSIONS` | 128 sessions | Prevents OS file descriptor and process table exhaustion. |

---

## 18. API Sketch (Public Rust Contract)

```rust
// crates/pursue-terminal/src/lib.rs

pub mod session;
pub mod command;
pub mod executor;
pub mod evidence;
pub mod service;
pub mod ipc;

pub use session::{Session, SessionId, SessionStatus};
pub use command::{CommandRequest, ExecutionResult};
pub use executor::{CommandExecutor, MockExecutor, ProcessExecutor};
pub use evidence::{TerminalEvidenceCapturer, CapturedEvidence};
pub use service::TerminalService;
pub use ipc::TerminalHandler;

// Representative core types:

pub struct SessionId(String);
impl SessionId {
    pub fn new(id: &str) -> Result<Self>;
    pub fn as_str(&self) -> &str;
}

pub struct CommandRequest {
    pub program: String,
    pub args: Vec<String>,
    pub working_dir: Option<PathBuf>,
    pub env_overrides: BTreeMap<String, String>,
    pub timeout_secs: Option<u64>,
    pub max_output_bytes: Option<usize>,
}

pub struct ExecutionResult {
    pub command_id: String,
    pub exit_code: Option<i32>,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub started_at_unix: u64,
    pub finished_at_unix: u64,
    pub timed_out: bool,
    pub truncated: bool,
}

pub trait CommandExecutor: Send + Sync {
    fn execute(&self, session: &Session, request: &CommandRequest) -> Result<ExecutionResult>;
}

pub struct TerminalEvidenceCapturer;
impl TerminalEvidenceCapturer {
    pub fn capture_output(
        case_store: &mut dyn CaseStore,
        case_id: &CaseId,
        session: &Session,
        result: &ExecutionResult,
        source_label: &str,
    ) -> Result<CapturedEvidence>;
}
```

---

## 19. Dependency Policy

`crates/pursue-terminal` relies strictly on existing workspace dependencies:
- `pursue-core` (error types, hex codecs)
- `pursue-evidence` (content addressing, audit logs, file stores)
- `pursue-case` (case containers, case store traits)
- `pursue-runtime` (configuration, logging, service lifecycle, IPC protocol)
- `serde`, `serde_json` (serialization)

**No external heavyweight dependencies (such as tokio, pty crates, or terminal UI crates) are added in Phase 1E.**

---

## 20. Definition of Done (Phase 1E)

Phase 1E implementation is complete when:
1. `crates/pursue-terminal` is established in the Cargo workspace with `#![deny(unsafe_code)]` and `#![warn(missing_docs)]`.
2. Session and command data models are fully implemented, validated, and serialized.
3. `MockExecutor` and `ProcessExecutor` are implemented and tested.
4. Output evidence capture directly stores verified blobs via `EvidenceStore` and attaches them to `Case` via `CaseStore`.
5. `TerminalService` runs through `pursue-runtime` lifecycle (`init -> start -> run -> shutdown`).
6. `TerminalHandler` routes IPC requests via `pursue-runtime::ipc::Router`.
7. Unit and integration tests pass on Windows and Linux CI (100% pass rate).
8. `cargo fmt`, `cargo clippy --workspace --all-targets -- -D warnings`, and release builds pass cleanly.

---

## 21. Ordered Implementation Plan

Phase 1E will be implemented in subsequent prompts in small, testable increments:

1. **Step 1: Workspace & Crate Scaffold**  
   Add `crates/pursue-terminal` to root `Cargo.toml`; configure dependencies and workspace lints.
2. **Step 2: Session & Command Models**  
   Implement `SessionId`, `Session`, `SessionStatus`, `CommandRequest`, and `ExecutionResult` with validation.
3. **Step 3: Command Execution Trait & Mock Executor**  
   Define `CommandExecutor` trait; implement `MockExecutor` for deterministic testing.
4. **Step 4: Process Executor (Real OS Processes)**  
   Implement `ProcessExecutor` with stream capture, output truncation (`MAX_OUTPUT_BYTES`), and timeout enforcement.
5. **Step 5: Terminal Evidence Capture Layer**  
   Implement `TerminalEvidenceCapturer` linking process output to `EvidenceStore::put` and `Case::attach`.
6. **Step 6: Terminal Runtime Service**  
   Implement `TerminalService` adhering to `pursue_runtime::service::Service`.
7. **Step 7: Terminal IPC Dispatcher**  
   Implement `TerminalHandler` for IPC `Router` dispatch.
8. **Step 8: End-to-End Integration Tests**  
   Build `tests/terminal_integration.rs` testing session -> execution -> evidence capture -> case integrity.
9. **Step 9: Linux Cross-Compilation & Final Validation**  
   Verify `cargo check --target x86_64-unknown-linux-gnu`, `cargo fmt`, `clippy -D warnings`, and full workspace test suite.
