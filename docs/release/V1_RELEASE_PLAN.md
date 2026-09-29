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
  - Phase 6 / Milestone 7: ISO Build + Real Boot Validation (`build/scripts/`, `crates/pursue-desktop`, `tests/qemu_verify_boot.py`) — native Debian 13 (Trixie) rootfs build (`live-boot`), hybrid dual UEFI/BIOS ISO generation via `xorriso`/`grub-mkrescue` (626MB, SHA-256 `f085f4497a4643c19bd998c0140508f2973bf391aba34a29ad08f31e924422aa`), automated ISO self-check (9/9 checks passed), headless systemd daemon integration with mode `0770` Unix domain socket, automated SeaBIOS & UEFI boot validation in QEMU, verified system accounts (`pursue`, `pursue-investigator`), deployment directory permissions, and successful 7-step live investigation flow test (disposable case, live terminal execution, CAS evidence capture, audit chain verification, report export, and deep cryptographic case integrity check) (`docs/development/ISO_BUILD_AND_BOOT_VALIDATION.md`)
  - Milestone 8: V1 Beta Hardening + Adversarial Security Validation (`crates/pursue-case`, `crates/pursue-evidence`, `crates/pursue-runtime`, `crates/pursue-desktop`, `build/systemd/`) — strict path traversal prevention on CaseId & Report export, atomic temporary write + fsync + atomic rename persistence on evidence blobs and case manifests, multithreaded concurrent IPC request serving with non-blocking per-dispatch lock acquisition, systemd security sandbox hardening (`ProtectClock`, `ProtectHostname`, `RestrictSUIDSGID`, `CapabilityBoundingSet=`, `DevicePolicy=closed`, `RestrictAddressFamilies`), 19/19 adversarial attack simulation test suite passing, rebuilt static musl release binary and live bootable ISO (SHA-256 `de6c5bcb684d30c6df233a10ed17010213813ccfbf02008ab70d9cbd537c2faa`), verified full 7-step live investigation flow in QEMU under hardened daemon, 348/348 workspace tests passing, clippy clean, fmt clean.
  - Phase 9: V1 Beta Product Integration & First-Boot Readiness (`crates/pursue-desktop`, `build/config/`, `build/scripts/`) — Unix domain socket IPC client (`SocketClient`) with 2s retry window for daemon startup, dynamic investigator identity from `USER`/`LOGNAME` environment variable, IPC health indicator in dashboard and settings, unique session IDs for terminal/browser, session termination controls, report verification auto-fill, Sway compositor autostart (`/etc/sway/config.d/00-pursue.conf`), console autologin via `getty@tty1.service.d/autologin.conf`, profile.d session launcher (`/etc/profile.d/00-pursue-session.sh`), deterministic startup chain validated (kernel → systemd → pursue-runtime → IPC socket → getty autologin → sway → pursue-desktop → SocketClient), rebuilt static musl binary and live ISO (SHA-256 `10b3c00730e21d4ea757a28a539b377e816d5b7e63d369192b6a62aa91cedf38`, 626 MB), QEMU boot validation passed, live investigation flow 7/7 passed, 348/348 workspace tests passing, clippy clean, fmt clean. **ISO is ready for first manual boot by the owner.**
  - Phase 10: V1 Beta Release Candidate & Manual Boot Readiness (`docs/release/`) — full baseline audit of startup chain from source, systemd hardening regression check, Sway/autologin/profile.d chain validation, desktop first-paint and IPC readiness audit, error/failure-path review, manual-boot readiness checklist (`docs/release/MANUAL_BOOT_TEST_CHECKLIST.md`), fresh static musl binary rebuild, final release candidate ISO (SHA-256 `365fdc9c237d46c03e0a76af09fbead6c14b938c0fcf56bbd2b1d5f669083752`, 655,984,640 bytes), ISO self-check 9/9 passed, QEMU cold-boot validated (`systemctl is-system-running` returns `running`), live investigation flow 7/7 passed (case creation, terminal execution, CAS evidence capture, audit chain verification, report export, deep cryptographic integrity). **FINAL V1 BETA ISO READY FOR OWNER MANUAL TEST.**
- **Subsequent:**
  - Advanced graph visualization UI
  - Hardware installer integration on target Linux image


