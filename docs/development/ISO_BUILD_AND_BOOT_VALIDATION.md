# PURSUE OS — Phase 6: ISO Build & Boot Validation Foundation

> Status: **Implemented (build scripts hardened; ISO build and boot validation require Linux host)**
> Depends on: Phase 5 (OS Integration)
> Locked decisions referenced: A-001, A-002, A-011

## Build Environment Contract

### Requirements

| Requirement | Specification |
|-------------|---------------|
| Host OS | Linux (Debian 12+, Ubuntu 22.04+, or WSL2) |
| Root access | Required for debootstrap and chroot |
| Target arch | amd64 (x86_64) |
| Debian release | trixie (default) or bookworm |
| Rust target | `x86_64-unknown-linux-gnu` |
| Build tools | `debootstrap`, `mksquashfs`, `xorriso`, `grub-efi-amd64-bin` |

### Build Workflow

```
1. cargo build --workspace --release --target x86_64-unknown-linux-gnu
2. sudo build/scripts/build-base.sh [target-dir]
3. sudo build/scripts/build-iso.sh [rootfs-dir] [output.iso]
4. build/scripts/validate-iso.sh [output.iso]
```

### Windows Development Note

The development machine runs Windows. Build scripts require Linux and cannot be
executed natively. The ISO build and QEMU boot validation steps are explicitly
**pending until a Linux build environment is available**.

What CAN be validated on Windows:
- Rust workspace: `cargo test --workspace`
- Linux cross-check: `cargo check --target x86_64-unknown-linux-gnu`
- Build config: `validate-build-config.sh` (runs in Git Bash)
- All integration tests pass

What CANNOT be validated on Windows:
- `debootstrap` rootfs construction
- SquashFS + ISO generation
- QEMU boot validation
- Systemd service startup
- User/group creation

**DO NOT fabricate ISO build success on Windows.**

## Build Script Hardening (Phase 6)

### build-base.sh

| Hardening | Description |
|-----------|-------------|
| `set -euo pipefail` | Strict error handling |
| Root privilege check | Exits if not root |
| Binary existence check | Verifies `pursue-desktop` exists before build |
| Cross-compile fallback | Checks `x86_64-unknown-linux-gnu/release/` then `release/` |
| Reports directory | Added to rootfs layout |
| Ownership step | Sets `pursue:pursue-investigator` on data directories |

### build-iso.sh

| Hardening | Description |
|-----------|-------------|
| `set -euo pipefail` | Strict error handling |
| Root privilege check | Exits if not root |
| Cleanup trap | Removes work directory on exit |
| Binary validation | Checks pursue-desktop exists in rootfs |
| Missing kernel = error | Changed from WARNING to hard ERROR |
| SHA-256 checksum | Generated alongside ISO |

### validate-build-config.sh

| Check | Description |
|-------|-------------|
| File presence | All build manifests, units, configs |
| A-011 compliance | No pursue-terminal.service or pursue-browser.service |
| ExecStart validation | Both units reference pursue-desktop binary |
| Security hardening | ProtectSystem + NoNewPrivileges in runtime unit |
| Package count | Minimum 20 packages in manifest |
| Sway presence | Decision A-002 compositor check |
| Sysusers completeness | pursue + pursue-investigator defined |
| Tmpfiles completeness | All deployment directories including /reports |
| Config template | Required deployment paths present |

## ISO Self-Check (validate-iso.sh)

Automated post-build validation script. Checks:

1. **ISO file existence** — file must exist
2. **Minimum size** — at least 50MB (bare Debian + kernel + PURSUE binary)
3. **SHA-256 checksum** — checksum file exists and matches
4. **SquashFS** — `filesystem.squashfs` present in ISO
5. **GRUB config** — `grub.cfg` present
6. **Kernel** — `vmlinuz` present
7. **Initramfs** — `initrd.img` present
8. **Volume label** — `PURSUE_OS_V1`

## QEMU Boot Validation Harness

### Procedure (requires Linux host)

```bash
# Launch ISO in QEMU
qemu-system-x86_64 \
    -m 2048 \
    -cdrom target/pursue-os-v1-amd64.iso \
    -boot d \
    -nographic \
    -serial mon:stdio \
    -no-reboot
```

### Boot Validation Test Points

| # | Test | Expected |
|---|------|----------|
| 1 | GRUB menu appears | "PURSUE OS — Forensic Investigation Workstation" |
| 2 | Kernel boots | No kernel panic |
| 3 | systemd reaches multi-user.target | `systemctl is-system-running` = running |
| 4 | pursue user exists | `id pursue` succeeds |
| 5 | pursue-investigator user exists | `id pursue-investigator` succeeds |
| 6 | pursue-runtime.service active | `systemctl is-active pursue-runtime.service` |
| 7 | Tor service active | `systemctl is-active tor.service` |
| 8 | IPC socket exists | `/run/pursue/ipc.sock` present |
| 9 | Case directory exists | `/var/lib/pursue/cases` with correct permissions |
| 10 | Evidence directory exists | `/var/lib/pursue/profiles` with correct permissions |
| 11 | Reports directory exists | `/var/lib/pursue/reports` with correct permissions |
| 12 | Log directory exists | `/var/log/pursue` with correct permissions |
| 13 | Config file exists | `/etc/pursue/config.toml` is parseable |
| 14 | Binary exists | `/usr/lib/pursue/bin/pursue-desktop` executable |
| 15 | Binary runs headless | `pursue-desktop --headless` exits 0 |
| 16 | Tor SOCKS proxy | Port 9050 listening |
| 17 | Sway available | `which sway` succeeds |
| 18 | No direct internet | `curl --max-time 5 http://example.com` fails (nftables) |
| 19 | Journal logs clean | `journalctl -u pursue-runtime` has no FATAL errors |
| 20 | Hostname | `hostname` = pursue-os |
| 21 | No external AI API | No outbound AI API connections |

### Boot Failure Diagnostics

If boot validation fails, collect:

```bash
# Kernel messages
dmesg > /tmp/pursue-dmesg.log

# systemd journal
journalctl --no-pager > /tmp/pursue-journal.log

# Service-specific logs
journalctl -u pursue-runtime.service --no-pager > /tmp/pursue-runtime.log
journalctl -u tor.service --no-pager > /tmp/pursue-tor.log

# Failed units
systemctl --failed > /tmp/pursue-failed-units.log

# IPC socket status
ls -la /run/pursue/ > /tmp/pursue-ipc-status.log

# User verification
id pursue > /tmp/pursue-users.log 2>&1
id pursue-investigator >> /tmp/pursue-users.log 2>&1
```

## Current Validation Status

| Check | Status |
|-------|--------|
| Rust workspace tests | ✅ All passing |
| Linux cross-check | ✅ `cargo check --target x86_64-unknown-linux-gnu` |
| Build config validation | ✅ `validate-build-config.sh` |
| Integration tests (Phase 5) | ✅ 9 new tests |
| ISO build on Linux | ⏳ Pending Linux host |
| QEMU boot validation | ⏳ Pending Linux host |
