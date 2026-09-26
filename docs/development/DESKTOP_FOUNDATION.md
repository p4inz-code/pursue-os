# PURSUE OS — Desktop & IPC Foundation Specification (Phase 2)

> **Category:** B — Implementation documentation (maintained with the code).  
> **Status:** Authoritative specification for Phase 2; implementation crate `crates/pursue-desktop`.  
> **Scope:** Phase 2 Desktop and IPC Foundation: graphical investigator desktop shell, client IPC boundary, case management views, interactive terminal console, browser integration console, and service health monitoring.

---

## 1. Purpose & Design Principles

The **PURSUE OS Desktop** (`crates/pursue-desktop`) is the integrated graphical shell connecting the human investigator to the headless OS foundations.

### Cardinal Architectural Principles
1. **IPC-Only Boundary**:
   - The UI **NEVER** interacts directly with disk files, raw sqlite, or internal backend crate structs.
   - All state mutations and queries flow across the strict IPC protocol (`pursue_runtime::ipc::protocol::Request` / `Response`).
2. **Professional Investigator UX**:
   - Modern, focused, utilitarian, and distraction-free (dark/light neutral theme).
   - NOT a cyberpunk or green-terminal aesthetic (`docs/core/MASTER_SPEC.md` §4).
   - Clean typographical hierarchy, structured JSON inspectors, and prominent routing indicators.
3. **Defense in Depth**:
   - The UI is an unprivileged client; it is not a trusted security enforcement boundary.
   - The backend services enforce case isolation, input validation, output limits, and fail-closed Tor boundaries.

---

## 2. Desktop Architecture & Component Topology

```
                  ┌─────────────────────────────────────────┐
                  │       pursue-desktop (eframe/egui)       │
                  │  ┌───────────┐ ┌───────────┐ ┌────────┐ │
                  │  │ Case View │ │ Term View │ │ Web UI │ │
                  │  └─────┬─────┘ └─────┬─────┘ └───┬────┘ │
                  │        │             │           │      │
                  │  ┌─────┴─────────────┴───────────┴────┐ │
                  │  │         DesktopState / Controller  │ │
                  │  └───────────────────┬────────────────┘ │
                  │                      │                  │
                  │  ┌───────────────────┴────────────────┐ │
                  │  │          IpcClient Engine          │ │
                  │  └───────────────────┬────────────────┘ │
                  └──────────────────────┼──────────────────┘
                                         │  IPC Request / Response
                                         ▼  (/run/pursue/ipc.sock)
                  ┌─────────────────────────────────────────┐
                  │         pursue-runtime IPC Router       │
                  │     ├── Handler: "terminal"             │
                  │     ├── Handler: "browser"              │
                  │     └── Handler: "case"                 │
                  └─────────────────────────────────────────┘
```

---

## 3. UI Modules & Capabilities

### A. Case Management View
- **Case Listing & Creation**: Allows creating new cases (`case.create`) with title, investigator metadata, and directory.
- **Active Case Context**: Selecting an active case binds the current terminal and browser sessions to that case ID.
- **Evidence Reference List**: Displays content-addressed SHA-256 evidence blobs attached to the active case.
- **Audit Log Inspector**: Shows the tamper-evident hash-chained audit entries with timestamps, actors, and actions.

### B. Investigation Terminal Console
- **Session Control**: Creates and binds an execution session to the active case.
- **Command Dispatch**: Accepts executable name and discrete arguments (no shell injection).
- **Stream Visualization**: Displays separate, color-coded stdout and stderr outputs.
- **Status Indicators**: Shows exit code, elapsed duration (ms), timeout warnings, and truncation flags.
- **One-Click Evidence Capture**: Triggers `terminal.evidence.capture` to store output into the case's evidence store.

### C. Investigation Browser Console
- **Session Control**: Creates an isolated session bound to the active case.
- **Routing Mode Switcher**: Explicit toggle between `Direct` and `Tor` modes.
- **Fail-Closed Warning**: If Tor mode is active and Tor proxy is unreachable, displays a prominent red warning; direct networking is blocked.
- **URL Navigation**: Validates URL format (`http`/`https` only) and executes navigation requests over IPC.
- **Response Inspector**: Displays HTTP status code, safe/scrubbed headers, and response body text/HTML.
- **One-Click Web Evidence Capture**: Triggers `browser.evidence.capture` to store web content or headers into the case.

### D. Service Health & Status Bar
- Shows connection health to `terminal`, `browser`, and `case` services.
- Displays current investigator actor ID and active case name.

---

## 4. Security & Privilege Invariants

1. **Unprivileged Process Execution**:
   - `pursue-desktop` runs as the non-root `investigator` user.
   - Sockets and files are accessed under the `pursue-investigator` group permissions.
2. **Credential Redaction in UI**:
   - The UI automatically redacts sensitive authorization and cookie headers when displaying HTTP response metadata.
3. **No Direct Storage Mutation**:
   - All evidence blobs are written by the backend evidence store, preserving cryptographic hashing and audit integrity.
