# PURSUE OS — Phase 5: OS Integration Specification

> Status: **Implemented**
> Depends on: Phases 1A–4 (all complete)
> Locked decisions referenced: A-002, A-005, A-006, A-007, A-008, A-009, A-010, A-011

## Objective

Move PURSUE OS from "application + OS build infrastructure" to one coherent
installable/bootable PURSUE OS system.

## Architecture Summary

### Decision A-011: Single Monolithic Binary

All domain services (terminal, browser, case, report) run **in-process** within
the single `pursue-desktop` binary. There are no separate `pursue-runtime`,
`pursue-terminal`, or `pursue-browser` executables.

| Mode | Command | systemd Unit | User |
|------|---------|--------------|------|
| Runtime daemon (headless) | `pursue-desktop --headless --config /etc/pursue/config.toml` | `pursue-runtime.service` | `pursue` |
| Desktop GUI | `pursue-desktop --config /etc/pursue/config.toml` | `pursue-desktop.service` | `pursue-investigator` |

### Service Dependency Graph

```
systemd
  ├── tor.service (debian-tor user, /usr/bin/tor)
  ├── pursue-runtime.service (pursue user, depends on tor)
  │     └── In-process handlers: case, terminal, browser, report
  └── pursue-desktop.service (pursue-investigator, depends on pursue-runtime)
```

### User & Account Model (Decision A-008)

| Account | UID | Home | Shell | Purpose |
|---------|-----|------|-------|---------|
| `pursue` | dynamic | `/var/lib/pursue` | `/usr/sbin/nologin` | System daemon — backend services |
| `pursue-investigator` | 1001 | `/home/pursue-investigator` | `/bin/bash` | Interactive investigator — desktop GUI |

Group `pursue-investigator` provides IPC socket read/write access.

## Filesystem Layout

```
/etc/pursue/config.toml           — System configuration
/usr/lib/pursue/bin/pursue-desktop — Single binary (A-011)
/run/pursue/                       — IPC socket directory (0770, pursue:pursue-investigator)
/var/lib/pursue/                   — Base data directory (0750, pursue:pursue-investigator)
/var/lib/pursue/cases/             — Case repositories (0770)
/var/lib/pursue/profiles/          — Browser profile storage (0770)
/var/lib/pursue/reports/           — Exported reports (0770)
/var/log/pursue/                   — Structured JSON logs (0750)
```

## Configuration Model

The runtime configuration (`/etc/pursue/config.toml`) is parsed by
`pursue_runtime::config::Config`. All path fields are optional and default to
subdirectories of `data_dir` via resolver methods.

```toml
version = 1
log_level = "info"
data_dir = "/var/lib/pursue"
ipc_socket_path = "/run/pursue/ipc.sock"
case_dir = "/var/lib/pursue/cases"
browser_profile_dir = "/var/lib/pursue/profiles"
report_dir = "/var/lib/pursue/reports"
log_file = "/var/log/pursue/runtime.log"
```

## Systemd Hardening

### pursue-runtime.service

| Directive | Value |
|-----------|-------|
| ProtectSystem | strict |
| ProtectHome | true |
| PrivateTmp | true |
| ProtectKernelTunables | true |
| ProtectKernelModules | true |
| ProtectControlGroups | true |
| MemoryDenyWriteExecute | true |
| RestrictRealtime | true |
| RestrictNamespaces | true |
| LockPersonality | true |
| NoNewPrivileges | true |

### pursue-desktop.service

| Directive | Value |
|-----------|-------|
| ProtectSystem | strict |
| ProtectHome | read-only |
| PrivateTmp | true |
| NoNewPrivileges | true |

## Security Invariants

1. **Tor fail-closed (A-006):** Tor unavailable → request fails. Never fallback to direct internet.
2. **Desktop zero-access (A-007):** Desktop has zero direct filesystem/process/database access. All interaction through IPC.
3. **No external AI (A-009):** No external AI API dependency in V1.
4. **Offline-first (A-010):** Must work without internet connectivity.
5. **Evidence immutability:** Content-addressed SHA-256. No silent mutation.
6. **Audit completeness:** Hash-chained audit log for all state changes.
7. **No secrets in logs:** Bearer tokens, passwords, cookies are redacted.

## Changes from Phase 1B

| Component | Before (Phase 1B) | After (Phase 5) |
|-----------|-------------------|-----------------|
| `pursue-runtime.service` ExecStart | `/usr/lib/pursue/bin/pursue-runtime` (non-existent) | `/usr/lib/pursue/bin/pursue-desktop --headless` |
| `pursue-terminal.service` | Existed (referenced non-existent binary) | **Removed** (in-process per A-011) |
| `pursue-browser.service` | Existed (referenced non-existent binary) | **Removed** (in-process per A-011) |
| `pursue-desktop.service` | Did not exist | **Created** |
| `packages.list` compositor | `weston` | `sway` (Decision A-002) |
| `sysusers.d` | `pursue-investigator` as group only | Added as user (UID 1001) |
| `tmpfiles.d` | Missing reports directory | Added `/var/lib/pursue/reports` |
| `Config` struct | 4 fields | 8 fields + 5 resolver methods |
| `main.rs` | Hardcoded temp dirs | Config-driven with `--config` flag |

## Test Coverage

Phase 5 adds 9 integration tests:

- `test_phase5_systemd_runtime_unit_references_pursue_desktop`
- `test_phase5_no_spurious_service_binaries`
- `test_phase5_desktop_service_unit_exists`
- `test_phase5_sysusers_defines_both_accounts`
- `test_phase5_tmpfiles_covers_all_storage_directories`
- `test_phase5_packages_list_includes_sway`
- `test_phase5_config_driven_service_initialization`
- `test_phase5_preset_file_consistency`
- Updated `test_bootable_base_config_validity` (expanded assertions)
