# PURSUE OS — Phase 6 & Milestone 7: ISO Build & Boot Validation

> Status: **VERIFIED & COMPLETE**
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
| Rust target | `x86_64-unknown-linux-musl` (fully static-pie binary) |
| Build tools | `debootstrap`, `mksquashfs`, `xorriso`, `grub-pc-bin`, `grub-efi-amd64-bin`, `live-boot` |

### Build Workflow

```bash
1. cargo build --workspace --release --target x86_64-unknown-linux-musl
2. sudo build/scripts/build-base.sh [target-dir]
3. sudo build/scripts/build-iso.sh [rootfs-dir] [output.iso]
4. build/scripts/validate-iso.sh [output.iso]
5. python3 tests/qemu_verify_boot.py
```

## Milestone 7 Verified Build Artifacts

| Artifact | Path / Specification | Verified Value |
|----------|----------------------|----------------|
| Live ISO Image | `target/pursue-os-v1-amd64.iso` | 655,972,352 bytes (~626 MB) |
| SHA-256 Checksum | `target/pursue-os-v1-amd64.iso.sha256` | `f085f4497a4643c19bd998c0140508f2973bf391aba34a29ad08f31e924422aa` |
| Kernel Version | Linux Debian kernel | `6.12.107+deb13-amd64` (SMP PREEMPT_DYNAMIC) |
| Boot Firmware Support | Hybrid BIOS (SeaBIOS) & UEFI (OVMF) | Dual-boot GRUB 2.14 rescue image |
| Live Rootfs Overlay | `live-boot` / SquashFS | `/live/filesystem.squashfs` mounted to RAM |
| Single Binary Core | `crates/pursue-desktop` | Static ELF x86_64, in-process IPC router & services |

## ISO Self-Check (validate-iso.sh)

Automated post-build validation script (`build/scripts/validate-iso.sh`):

1. **ISO file existence** — ✅ PASSED
2. **Minimum size** — ✅ PASSED (655,972,352 bytes >= 50MB)
3. **SHA-256 checksum** — ✅ PASSED (matches `f085f4497a4643c19bd998c0140508f2973bf391aba34a29ad08f31e924422aa`)
4. **SquashFS presence** — ✅ PASSED (`live/filesystem.squashfs` present in ISO)
5. **GRUB config presence** — ✅ PASSED (`boot/grub/grub.cfg` present)
6. **Kernel presence** — ✅ PASSED (`live/vmlinuz` present)
7. **Initramfs presence** — ✅ PASSED (`live/initrd.img` present)
8. **Volume label** — ✅ PASSED (`PURSUE_OS_V1`)
9. **Single binary packaging** — ✅ PASSED (`pursue-desktop` bundled in rootfs)

## Real QEMU Boot Validation Results

Automated execution via `tests/qemu_verify_boot.py` in QEMU:

| # | Test Check | Verified Result | Status |
|---|------------|-----------------|--------|
| 1 | BIOS (SeaBIOS) Boot | GRUB 2.14 menu loaded, default countdown triggered | ✅ PASS |
| 2 | UEFI (OVMF) Boot | UEFI firmware loads EFI/BOOT/BOOTX64.EFI, launches GRUB | ✅ PASS |
| 3 | Kernel Startup | Kernel `6.12.107+deb13-amd64` booted on serial `ttyS0` | ✅ PASS |
| 4 | Live Overlay Mount | `live-boot` mounts `/live/filesystem.squashfs` with tmpfs rw overlay | ✅ PASS |
| 5 | Systemd Multi-User | Systemd reaches multi-user target; `systemctl is-system-running` | ✅ PASS |
| 6 | Service Daemon Account | `id pursue` -> `uid=990(pursue) gid=990(pursue) groups=990,991` | ✅ PASS |
| 7 | Investigator Account | `id pursue-investigator` -> `uid=1001(pursue-investigator) groups=sudo,audio,video,input` | ✅ PASS |
| 8 | Passwordless Console Login | Login prompt on `ttyS0` logs into `pursue-investigator` shell | ✅ PASS |
| 9 | pursue-runtime.service | `active (running)`, PID 470 (`pursue-desktop --headless`) | ✅ PASS |
| 10 | Unix Domain Socket | `/run/pursue/ipc.sock` created mode `srwxrwx---` (`0770`) `pursue:pursue-investigator` | ✅ PASS |
| 11 | Deployment Directories | `/var/lib/pursue/cases`, `/profiles`, `/reports` mode `0770` | ✅ PASS |
| 12 | Logging Directory | `/var/log/pursue` mode `0750` | ✅ PASS |
| 13 | tor.service | `active (exited)` master instance active | ✅ PASS |
| 14 | Headless CLI Validation | `/usr/lib/pursue/bin/pursue-desktop --headless` prints config, exits 0 | ✅ PASS |

## End-to-End Live Investigation Flow Validation

Executed live over `/run/pursue/ipc.sock` via `pursue-desktop --verify-live`:

```
=== PURSUE OS Live Investigation Flow Verification ===
Connecting to IPC socket at /run/pursue/ipc.sock...
1. Creating disposable investigation case 'case-live-1790611850'...
   [OK] Case created: case-live-1790611850
2. Initializing terminal session 'term-live-1790611850'...
   [OK] Terminal session active
3. Executing live system command 'uname -a'...
   [OK] Command output: Linux pursue-os 6.12.107+deb13-amd64 #1 SMP PREEMPT_DYNAMIC Debian 6.12.107-1 (2026-08-29) x86_64 GNU/Linux
4. Capturing command output as immutable evidence artifact...
   [OK] Evidence captured with SHA-256 address: 2feaa8a3c4dc9e0b12cffb3a35f9f2d8100eb643be9a2d62e3fe1bd3a6949829
5. Inspecting hash-chained provenance audit log...
   [OK] Audit chain verified: true (2 events recorded)
6. Generating and exporting forensic report...
   [OK] Report exported: "/var/lib/pursue/reports/live-report-1790611850.json"
   [OK] Report SHA-256: 0e49248e7ff66cbd95d283e3bee5cba367eec55a67c1922c0a248141ee5b16ae
7. Verifying deep cryptographic case integrity...
   [OK] Manifest verified: true
   [OK] Audit chain verified: true
   [OK] Evidence blobs verified: 1
=======================================================
 LIVE INVESTIGATION FLOW TEST: PASSED
=======================================================
```

## Current Validation Summary

| Check | Status |
|-------|--------|
| Rust workspace tests | ✅ All passing (329+ tests) |
| Linux cross-check | ✅ `cargo clippy --target x86_64-unknown-linux-gnu` clean |
| Build config validation | ✅ `validate-build-config.sh` passed |
| Native Linux ISO build | ✅ `target/pursue-os-v1-amd64.iso` generated (626 MB) |
| ISO self-check | ✅ `validate-iso.sh` (9/9 checks passed) |
| QEMU BIOS & UEFI boot | ✅ Both firmware modes boot to userspace |
| systemd service startup | ✅ `pursue-runtime.service` active and operational |
| IPC domain socket | ✅ Mode `0770` with bidirectional framing verified |
| Full investigation flow | ✅ Live case, terminal, CAS capture, audit chain, export & verification PASSED |
