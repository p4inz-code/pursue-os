# PURSUE OS — V1 Release Plan

> **Source:** PURSUE OS master handoff (authoritative). Restored 2026-08-09.
> **Category:** A — Authoritative phase plan.

## Phase Sequence (from handoff)

- **Phase 1A — Technical architecture lock:** base Linux distribution, build system, package strategy, desktop environment, language/runtime choices, core service architecture, UI framework, data/storage architecture, security boundaries, test framework, CI strategy, ISO/build architecture. Research as needed; ask the user only for owner-level choices.
- **Phase 1B — Minimal bootable PURSUE base:** reproducible development/build environment; eventually a bootable minimal PURSUE image. Do not implement every feature at once.
- **Phase 1C — Core runtime:** core services, configuration, logging, secure storage foundation, IPC/service boundaries, update/package foundations, test harness.
- **Phase 1D — Case/evidence foundation:** case model, evidence model, provenance model, integrity model, secure storage, investigator-controlled metadata.
- **Phase 1E — Terminal foundation** (flagship phase).
- **Phase 1F — Browser + Tor foundation** (dedicated major phase; Tor integration in detail together with the terminal).

## Stop Conditions (do not begin before their phases)

- Full Investigation Terminal
- Full Investigation Browser
- Tor integration (as a completed feature)
- Large OSINT tool collection
- Advanced DFIR tooling
- Graph UI / reporting UI
- Plugin marketplace
- Advanced AI
- Production release / final ISO

## Current Progress

- **Completed:**
  - Repository / CI foundation
  - Foundation primitives (`crates/pursue-core`)
  - Evidence integrity foundation (`crates/pursue-evidence`)
  - Phase 1C: Core runtime (`crates/pursue-runtime`) — configuration, structured logging, service lifecycle, IPC boundary
  - Phase 1D: Case foundation (`crates/pursue-case`) — case identifier, case container, audited operations, case store layer with per-case isolation and integrity verification
  - Phase 1E: Terminal foundation (`crates/pursue-terminal`) — session lifecycle, execution abstractions (mock/process), bounded stream capture, evidence packaging, service & IPC routing
  - Phase 1F: Browser + Tor foundation (`crates/pursue-browser`) — session lifecycle, routing modes (direct/Tor), fail-closed Tor boundaries, profile isolation, navigation validation, web evidence capture orchestration, service & IPC routing
  - Phase 1B: Bootable Base Configuration Foundation (`build/`) — Debian package manifests, systemd service units, sysusers/tmpfiles privilege definitions, validation scripts, build specifications (`docs/development/BOOTABLE_BASE.md`)
  - Phase 2: Desktop & IPC Foundation (`crates/pursue-desktop`) — egui/eframe desktop shell, strict IPC client dispatch, Case management view, Investigation Terminal console view, Investigation Browser & Tor console view, diagnostics & integration test suite (`docs/development/DESKTOP_FOUNDATION.md`)
  - Phase 3: Investigation Workspace + Case Operations (`crates/pursue-case`, `crates/pursue-desktop`) — full case lifecycle (create, update title/notes, close, reopen, deep verify), investigation dashboard, evidence explorer, unified chronological investigation timeline, cryptographic hash-chain audit viewer (`docs/development/INVESTIGATION_WORKSPACE.md`)
  - Phase 4: Forensic Reporting + Export Foundation (`crates/pursue-report`, `crates/pursue-desktop`) — dedicated reporting crate, deterministic JSON generation, self-contained printable HTML reporting, cryptographic report sealing, path traversal protection, secret scrubbing, atomic file export, disk report verification (`docs/development/REPORTING_FOUNDATION.md`)
  - Phase 5: OS Integration (`build/`, `crates/pursue-runtime`, `crates/pursue-desktop`) — Decision A-011 single-binary reconciliation, `pursue-desktop --headless` runtime mode, `--config` CLI flag, expanded Config struct with deployment path resolvers, fixed systemd units (removed spurious terminal/browser services, added desktop service), corrected sysusers (pursue-investigator user entry), tmpfiles (reports directory), packages.list (sway per A-002), hardened build-base.sh with binary validation, 9 OS integration tests (`docs/development/OS_INTEGRATION.md`)
  - Phase 6: ISO Build + Boot Validation Foundation (`build/scripts/`) — hardened build-iso.sh (cleanup trap, binary validation, hard error on missing kernel), ISO self-check script (validate-iso.sh), updated validate-build-config.sh (A-011 compliance, sway check, ExecStart validation, config template check), QEMU boot validation harness with 21 test points, boot failure diagnostics documentation (`docs/development/ISO_BUILD_AND_BOOT_VALIDATION.md`)
- **Pending External Host Execution:**
  - Live ISO binary generation (`build-iso.sh` on Linux host with root/container privileges; see DECISION_RECORD B7)
  - QEMU boot validation (requires Linux host with KVM/QEMU)
- **Subsequent:**
  - Advanced graph visualization UI
  - Hardware installer integration on target Linux image
