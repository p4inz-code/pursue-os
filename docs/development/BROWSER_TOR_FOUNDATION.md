# PURSUE OS — Browser & Tor Foundation (Phase 1F Specification)

> **Category:** B — Implementation documentation (maintained with the code).  
> **Status:** Fully implemented and verified in Phase 1F (262/262 workspace tests passing, clippy clean, Linux cross-compile clean).  
> **Scope:** Phase 1F Investigation Browser and Tor Foundation: browser session management, profile isolation, routing modes (Direct vs. Tor), fail-closed network boundaries, navigation validation, web evidence capture orchestration, runtime service integration, and IPC boundary.

---

## 1. Investigation Browser Purpose

The **Investigation Browser** is one of the two flagship interface pillars of PURSUE OS (the other being the Investigation Terminal; see `docs/core/MASTER_SPEC.md` §4 and `docs/core/PURSUE_V1_CONTRACT.md` §2, §4).

### Why It Exists
Digital investigations across OSINT, SOCMINT, GEOINT, CTI, and DFIR require intensive web research. In generic operating systems:
- Browsing history, cookies, and cached data leak across unrelated cases.
- Captured web pages, raw HTML, and screenshots lack verifiable cryptographic provenance.
- Tor routing is often tacked on as an external, misunderstood third-party app with silent fallbacks to cleartext connections.
- Metadata and credentials are often leaked in browser cache files and sync stores.

PURSUE OS elevates the browser to an integrated, forensic investigation instrument where:
- Each browsing session is bound strictly to an active `CaseId`.
- Browser profile state (cookies, cache, session storage) is strictly isolated per session and case.
- Routing is explicitly controlled by the investigator: Direct mode or Tor mode, with zero silent fallback.
- Web artifacts (page responses, HTTP headers, downloads, screenshots) are hashed via SHA-256 and committed directly into the case's immutable content-addressed evidence store (`pursue-evidence`).
- Actions are recorded in the case's hash-chained `AuditLog` (`pursue-case`).

---

## 2. Browser Architecture

Phase 1F builds the headless foundation in a new workspace crate: `crates/pursue-browser`.

```
crates/pursue-browser
    ├── depends on ──> crates/pursue-runtime   (Config, Logger, Service, IPC Router)
    ├── depends on ──> crates/pursue-case      (Case, CaseId, CaseStore, FileCaseStore)
    ├── depends on ──> crates/pursue-evidence  (ContentAddress, EvidenceRecord, EvidenceStore)
    └── depends on ──> crates/pursue-core      (Error, Result, hex)
```

The browser foundation acts as the engine and supervisory controller for research browsing:
1. **Data Models:** Validated `BrowserSessionId`, `BrowserSession`, `RoutingMode`, `NavigationRequest`, `NavigationResult`, `WebArtifact`.
2. **Engine Abstraction:** Trait-based `BrowserEngine` supporting deterministic mock execution (`MockBrowserEngine`) and live network execution (`NetworkEngine`).
3. **Evidence Capture:** `WebEvidenceCapturer` converting web artifacts into `EvidenceRecord` blobs in `FileStore` and attaching them to `Case`.
4. **Service & IPC:** `BrowserService` implementing `pursue_runtime::service::Service` and `BrowserHandler` implementing `pursue_runtime::ipc::dispatch::Handler`.

---

## 3. Tor Integration Boundary

Tor provides onion routing to obscure transport-layer IP addresses and route to `.onion` hidden services. In PURSUE OS:
- Tor is an integrated platform subsystem accessed via a local SOCKS5 proxy (default `127.0.0.1:9050`) or local Tor control interface.
- Tor is explicitly user-controlled. It is never forced silently on all network requests.
- When Tor mode is requested, all outbound connections and DNS resolution MUST pass through the Tor proxy.

---

## 4. Network Isolation Model

Network routing is modeled as a strictly verified invariant:
- **`RoutingMode::Direct`**: Requests are routed directly across standard network interfaces. Tor proxying is NOT invoked.
- **`RoutingMode::Tor`**: Requests MUST route through the configured Tor proxy. If Tor is not running, proxy connection fails, or configuration is invalid, the engine **FAILS CLOSED**.

---

## 5. User-Controlled Tor Behavior & No Silent Fallbacks

PURSUE OS enforces the following cardinal rules:
1. **No Silent Fallback to Direct:** If an investigator requests `RoutingMode::Tor` and Tor is down or unreachable, the system MUST NOT silently retry over the direct connection. It must abort and report an explicit routing failure error.
2. **No Silent Fallback to Tor:** If an investigator requests `RoutingMode::Direct`, the system MUST NOT silently route through Tor, ensuring the investigator retains complete comprehension and control over their external network footprint.
3. **Observable State:** Every navigation result and evidence capture record explicitly registers the active `RoutingMode`.

---

## 6. Browser Profile Isolation

Profile contamination between cases or sessions compromises forensic integrity and operational security.
- Every `BrowserSession` operates in an isolated filesystem profile directory:
  `profiles/<case_id>/<session_id>/`
- Profile components (cookies, local storage, HTTP cache, and browsing state) are kept strictly within this boundary.
- Case A can NEVER read, inherit, or share profile storage with Case B.
- Session termination purges ephemeral state according to session policy.

---

## 7. Case Association

- A `BrowserSession` can only be created by specifying a valid, existing `CaseId`.
- The session lifecycle and all evidence generated are irrevocably tied to that case.
- Cross-case evidence writes or session reassignments are strictly prohibited and prevented at the API boundary.

---

## 8. Evidence Capture

When the investigator captures a web page or resource:
1. The raw payload bytes (HTML body, downloaded file, or screenshot image data) are extracted.
2. The payload is hashed via SHA-256 (`ContentAddress::hash(&bytes)`).
3. The blob is stored into the case's evidence repository via `EvidenceStore::put`.
4. The resulting `ContentAddress` is attached to the `Case` via `Case::attach`.
5. An audit entry (`case.evidence.attached`) is added to the case's hash-chained `AuditLog`.
6. The updated `case.json` manifest is written to disk with verified manifest hashing.

---

## 9. Provenance Model

Every captured web artifact links:
- `timestamp_unix`: Time of capture.
- `actor`: Investigator identifier.
- `source_label`: Structured URI: `browser://session/<session_id>/url/<encoded_url>?mode=<mode>&status=<status>`.
- `case_id`: The owning case.
- `address`: The SHA-256 digest of the raw content.

### Evidentiary Truth Boundary
The cryptographic hash guarantees that *what the browser received at the moment of capture* has not been tampered with since collection. **It does NOT prove that the remote web server or website content is objective real-world truth.**

---

## 10. Screenshots, Page Artifacts, and Downloads

The browser foundation supports capturing distinct artifact categories:
- **`PageContent`**: Raw HTTP response body (HTML, JSON, XML, text).
- **`HttpHeaders`**: Canonicalized HTTP headers (excluding sensitive authentication credentials).
- **`Screenshot`**: Visual raster capture (PNG/JPEG) of rendered content.
- **`DownloadedFile`**: Raw binary streams downloaded during session research.

---

## 11. URL Handling & Security

Input URLs are untrusted user/network inputs. Validation rules:
- **Permitted Schemes:** `http://` and `https://` only.
- **Forbidden Schemes:** `file://`, `data:`, `javascript:`, `vbscript:`, `blob:`, `about:`, `chrome:`, `gopher:`, `ftp:`. Any attempt to navigate to a forbidden scheme is rejected with `Error::InvalidInput`.
- **Format Validation:** URLs must parse valid authority/host and path structures without NUL bytes or unencoded control characters.
- **Path Traversal & SSRF Defense:** Local file access via the browser navigation layer is strictly forbidden.

---

## 12. HTTP & Network Metadata Handling

- Metadata includes HTTP status code, response headers, content type, elapsed request duration, and server IP (when available and permitted by routing mode).
- Headers are preserved in canonical order for evidentiary consistency.
- Metadata is attached to the capture record.

---

## 13. DNS Considerations & Leak Prevention

- In `RoutingMode::Direct`, DNS resolution occurs via system resolvers.
- In `RoutingMode::Tor`, DNS resolution MUST be performed remotely through the SOCKS5 proxy (`SOCKS5h` protocol), ensuring local DNS queries do NOT leak the destination host to local network observers or ISPs.
- The browser foundation validates that hostnames are never resolved locally prior to proxy dispatch when Tor mode is active.

---

## 14. Cookies & Session Storage

- Cookies and session storage are confined strictly to the session's isolated profile.
- By default, session cookies and authorization tokens are NOT written to evidence records.
- Storing evidence of cookies requires an explicit, investigator-controlled action.

---

## 15. Credential & Secret Handling

- Passwords, `Authorization` headers, `Cookie` headers containing session tokens, and proxy credentials MUST be scrubbed from default provenance labels and logs.
- The `pursue-runtime::log::redact` module is used when emitting diagnostic logs containing URLs with query parameters or basic auth fragments.

---

## 16. Local vs. Tor Browsing Distinction

| Property | `RoutingMode::Direct` | `RoutingMode::Tor` |
| :--- | :--- | :--- |
| **Transport** | Direct TCP via host network stack | SOCKS5 proxy via Tor daemon |
| **DNS Resolution** | Local/Host DNS | Remote SOCKS5h DNS via Tor exit |
| **Hidden Services** | Rejected (`.onion` unreachable) | Permitted (`.onion` supported) |
| **Failure Behavior** | Fails on host net error | Fails closed on Tor proxy error |
| **Silent Fallback** | Prohibited | Prohibited |

---

## 17. Failure Behavior

- Tor proxy unreachable $\rightarrow$ Fail closed with `Error::ServiceFailure("Tor proxy unavailable")`.
- Invalid URL scheme $\rightarrow$ Fail closed with `Error::InvalidInput("forbidden URL scheme")`.
- Session terminated $\rightarrow$ Reject navigation requests with `Error::InvalidInput("session is terminated")`.
- Case not found $\rightarrow$ Fail with `Error::NotFound("case does not exist")`.
- Tampered evidence blob $\rightarrow$ Fail closed on load with `Error::IntegrityViolation`.

---

## 18. Offline & Non-Tor Behavior

- In offline or air-gapped environments, the system functions normally for local case management, inspection of existing evidence, and running mock-based test suites.
- Attempts to navigate when the network is offline produce clean, typed errors without crashing or corrupting case manifests.

---

## 19. Security Boundaries

- `#![deny(unsafe_code)]` and `#![warn(missing_docs)]` enforced across `crates/pursue-browser`.
- Zero shell commands executed for browsing.
- Memory-safe stream reading with buffer caps (default 16 MiB, max 64 MiB).
- Absolute separation between Case A and Case B storage directories.

---

## 20. IPC & Service Architecture

`BrowserService` integrates as a managed service within `pursue-runtime`:
- **`init`**: Validates configuration and storage directories.
- **`start`**: Prepares session registry and proxy supervisor.
- **`run`**: Supervises active sessions.
- **`shutdown`**: Terminates all active sessions cleanly.

`BrowserHandler` exposes the following IPC methods:
- `browser.session.create`: `{ "session_id": "...", "case_id": "...", "actor": "...", "mode": "direct"|"tor" }`
- `browser.session.get`: `{ "session_id": "..." }`
- `browser.session.terminate`: `{ "session_id": "..." }`
- `browser.navigate`: `{ "session_id": "...", "url": "..." }`
- `browser.evidence.capture`: `{ "session_id": "...", "artifact": "page"|"screenshot"|"download", ... }`

---

## 21. Testing Strategy

1. **Unit Tests (Offline & Deterministic):**
   - Session creation, status transitions, and ID validation.
   - URL parsing, valid schemes (`http`/`https`), and rejection of forbidden schemes (`file`, `javascript`, `data`).
   - Mock browser engine testing: direct mode success, Tor mode success, Tor unavailable failure, fail-closed assertions.
   - Proof that Tor mode never falls back to direct, and direct mode never falls back to Tor.
   - Artifact creation, provenance generation, and secret header scrubbing.
   - Profile path generation and directory isolation.
2. **Integration Tests (`tests/browser_integration.rs`):**
   - End-to-end: Session creation $\rightarrow$ navigation $\rightarrow$ evidence capture $\rightarrow$ case attachment $\rightarrow$ audit log verification.
   - Corrupted evidence detection failing closed on case load.
   - Multi-case isolation: Evidence in Case A cannot be retrieved from Case B.
   - Full IPC dispatch round-trip over `BrowserHandler`.
   - Real network engine test (when online or local loopback HTTP server).

---

## 22. Linux / Windows Development Constraints

- Cross-platform compilation: Builds and passes clippy and tests on both Windows and Linux (`x86_64-unknown-linux-gnu`).
- Offline CI capability: Unit and integration tests must run without requiring an external internet connection or a running live Tor daemon on host machines. Deterministic mock engines and loopback servers ensure 100% CI pass rate.

---

## 23. Dependencies

- `pursue-core`: Common errors and hex codecs.
- `pursue-evidence`: Content addressing and immutable evidence storage.
- `pursue-case`: Case management and hash-chained audit logs.
- `pursue-runtime`: Logging, service lifecycle, IPC dispatch.
- `serde`, `serde_json`: Serialization.
- Standard library networking (`std::net`).
- No external heavyweight browser/engine dependencies (e.g. no Chromium binary or WebKit bindings in Phase 1F core).

---

## 24. Explicit Non-Goals (Out of Scope for Phase 1F)

- **No full GUI browser window:** No WebKit/Chromium graphical rendering engine or browser UI window. Phase 1F is the headless core engine.
- **No full Tor daemon implementation from scratch:** PURSUE integrates with standard Tor binaries (`tor` daemon / SOCKS5 / control port), not a custom Rust onion router.
- **No claiming complete anonymity:** Tor routing obscures source IP and enables `.onion` routing, but does NOT guarantee absolute anonymity against sophisticated state adversaries, browser fingerprinting, or investigator error.
- **No automatic credential syncing:** No cloud passwords, Google accounts, or external telemetry sync.

---

## 25. Definition of Done (Phase 1F)

1. `crates/pursue-browser` added to the root Cargo workspace with `#![deny(unsafe_code)]` and `#![warn(missing_docs)]`.
2. Browser session model (`BrowserSessionId`, `BrowserSession`, `RoutingMode`) implemented with validation.
3. URL validator implemented with strict scheme and path safety.
4. Trait-based `BrowserEngine` with `MockBrowserEngine` and real network execution implemented.
5. Fail-closed Tor boundary verified: Tor failures never fall back to direct browsing.
6. Web evidence capture integrated with `pursue-evidence::EvidenceStore` and `pursue-case::FileCaseStore`.
7. `BrowserService` and `BrowserHandler` integrated with `pursue-runtime`.
8. Complete test suite passes on both Windows and Linux targets (100% pass rate).
9. Zero clippy warnings, zero fmt violations, clean release build.
