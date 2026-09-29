# PURSUE OS

> **Current Status: V1 Beta — Development Complete, Hardware / Manual Testing in Progress**
>
> PURSUE OS has completed its core V1 implementation and automated validation. The system has been built into a bootable hybrid ISO and verified under automated QEMU cold-boot testing. Real-world manual and hardware testing is currently underway. This is a testing/beta build, not a finalized production release.

---

**PURSUE OS** is an investigation-focused Linux operating system designed around evidence integrity, cryptographic provenance, privacy, local-first operation, and an integrated investigator workflow.

It is built for open-source intelligence (OSINT), digital forensics and incident response (DFIR), investigative research, intelligence analysis, and evidence-driven workflows.

Built and maintained by **P4inz**.

---

## What PURSUE OS Is

PURSUE OS is an integrated forensic investigation environment designed around a single core principle:

> **One coherent operating system for the complete investigation lifecycle.**

Rather than packaging an uncurated collection of independent third-party utilities, PURSUE OS integrates the primary stages of an investigation into a unified, cryptographically verified desktop shell:

- **Case Management:** Isolated case containers with audited operations and cryptographic manifests.
- **Investigation Terminal:** Purpose-built console environment with bounded execution, structured stream capture, and automatic case provenance.
- **Investigation Browser & Tor:** Configurable investigation browser with enforced fail-closed Tor network routing, remote SOCKS5h DNS resolution, and single-click evidence capture.
- **Evidence Vault:** Content-addressed storage (CAS) using SHA-256 addresses, strict immutable disk semantics, and write-once persistence.
- **Cryptographic Provenance:** Append-only hash-chained audit trails recording every investigator action and evidence ingestion event.
- **Forensic Reporting & Export:** Deterministic, sealed JSON and printable self-contained HTML forensic reports with automated cryptographic verification.

### What PURSUE OS Is Not

- **Not a generic hacking or penetration-testing distribution** (e.g., Kali Linux).
- **Not an absolute anonymity guarantee** (Tor and network isolation reduce attribution risks, but opsec requires investigator diligence).
- **Not an "AI operating system"** (AI features, if enabled, are optional, assistant-only, and strictly local; AI is never an authority and never silently mutates evidence).
- **Not an accredited forensic certification platform** (evidence structures follow scientific and cryptographic best practices, but legal admissibility depends on judicial jurisdiction).
- **Not yet a production-certified stable release** (currently in V1 beta hardware validation).

---

## Current Release Candidate ISO

| Parameter | Specification |
|---|---|
| **Candidate ISO Path** | `target/pursue-os-v1-amd64.iso` |
| **Exact Byte Size** | `403,159,040` bytes (~385 MB) |
| **Primary Checksum (SHA-256)** | `749263f12c4375ccb4c9079dc05b8c73c4586c9ba971422ebd19d8331776184a` |
| **Phase 10 Baseline Checksum** | `365fdc9c237d46c03e0a76af09fbead6c14b938c0fcf56bbd2b1d5f669083752` |
| **Target Architecture** | `x86_64` (AMD64, generic `x86-64-v1` baseline) |
| **Firmware Boot Mode** | Hybrid BIOS (MBR El Torito) + UEFI (GPT ESP) |
| **Base Distribution** | Debian 13 (Trixie) Minimal Base |
| **Kernel Version** | Linux `6.12.107+deb13-amd64` |
| **Volume Label** | `PURSUE_OS_V1` |
| **Desktop Environment** | Sway 1.10.1 (Wayland compositor) + `pursue-desktop` |
| **Default User Account** | `pursue-investigator` (passwordless login on tty1, sudo enabled) |

> [!NOTE]
> The ISO is a locally generated test artifact built directly from the verified source tree. It is not currently hosted as a precompiled binary download on GitHub releases.

---

## Validation & Verification Summary

The PURSUE OS codebase has undergone rigorous automated testing, security audit simulation, and live virtual machine cold-boot verification:

| Validation Gate | Verified Status | Evidence |
|---|---|---|
| **Workspace Test Suite** | ✅ **348/348 PASS** (Phase 9) / **310/310 PASS** (Phase 10) | All unit and integration tests across 8 crates pass cleanly. |
| **Adversarial Security Suite** | ✅ **19/19 PASS** | Path traversal, blob tampering, manifest tampering, hash break, and framing attacks fail closed. |
| **Code Formatting** | ✅ **CLEAN** | `cargo fmt --all -- --check` passes with zero warnings. |
| **Static Analysis (Clippy)** | ✅ **CLEAN** | `cargo clippy --workspace --all-targets -- -D warnings` clean. |
| **Linux Cross-Compilation** | ✅ **CLEAN** | `cargo check --target x86_64-unknown-linux-musl` clean. |
| **Build Configuration Audit** | ✅ **0 ERRORS** | `validate-build-config.sh` verifies units, users, tmpfiles, and packages. |
| **ISO Image Self-Check** | ✅ **9/9 PASS** | `validate-iso.sh` verifies size, checksum, GRUB, kernel, initrd, squashfs, and label. |
| **QEMU Cold Boot** | ✅ **PASS** | Kernel loads, systemd reaches `running` status (not degraded). |
| **Daemon & IPC Socket** | ✅ **PASS** | `pursue-runtime.service` active; `/run/pursue/ipc.sock` created mode `0770`. |
| **Tor Service** | ✅ **PASS** | Multi-instance Tor service initializes cleanly. |
| **Console Autologin** | ✅ **PASS** | `getty@tty1` autologins `pursue-investigator` without password prompts. |
| **Live Investigation Flow** | ✅ **7/7 PASS** | Case creation, terminal command execution, evidence CAS capture, audit verification, report export, and deep cryptographic case verification passed inside QEMU. |

---

## Hardware Testing Status

> [!IMPORTANT]
> **Distinction Between Automated Verification and Physical Hardware Testing:**
>
> - **Automated Testing:** **PASS** (100% of workspace tests and adversarial security tests).
> - **Virtualization (QEMU):** **PASS** (clean boot, service startup, IPC, and live flow).
> - **Physical Hardware:** **PENDING / IN PROGRESS**.

### Initial Physical Validation Target
The primary physical hardware validation machine is an **NVIDIA RTX 5060 / RTX 5070 Ti desktop system**.

Physical testing may uncover machine-specific differences, including:
- GPU driver and Wayland modesetting behavior (especially newer Blackwell architectures).
- Motherboard UEFI and Secure Boot implementation quirks.
- NVMe, SATA, and USB 3.0 controller timing.
- Wi-Fi and Ethernet hardware initialization.
- Monitor resolution and multi-display detection.

To maximize first-boot success on physical hardware, the following mitigations are baked into the ISO:
1. **Sway NVIDIA Compatibility:** Launched with `sway --unsupported-gpu` to bypass proprietary driver blocks.
2. **Software Cursor Emulation:** `WLR_NO_HARDWARE_CURSORS=1` prevents invisible mouse pointers.
3. **Software Renderer Fallback:** `WLR_RENDERER_ALLOW_SOFTWARE=1` and `mesa-vulkan-drivers` (lavapipe) ensure the UI renders even if GPU 3D acceleration is unavailable.
4. **GRUB Safe Graphics:** Dedicated boot entry `modprobe.blacklist=nouveau nouveau.modeset=0` for clean UEFI framebuffer fallback.
5. **Display Failure Trap:** If Sway fails, the system drops cleanly to a diagnostic shell rather than a crash loop.

For full hardware analysis, see [`docs/release/HARDWARE_COMPATIBILITY_AUDIT.md`](docs/release/HARDWARE_COMPATIBILITY_AUDIT.md).

---

## Owner Manual Testing Procedure

The owner's manual boot and workflow validation instructions are documented in:

👉 [`docs/release/MANUAL_BOOT_TEST_CHECKLIST.md`](docs/release/MANUAL_BOOT_TEST_CHECKLIST.md)

The 31-step checklist validates the end-to-end user experience on real hardware:
1. **Boot:** BIOS / UEFI GRUB menu selection.
2. **Login:** Automatic passwordless login into `pursue-investigator`.
3. **Session:** Automatic launch of Sway and `pursue-desktop`.
4. **IPC:** Automatic connection to `/run/pursue/ipc.sock` (status indicator shows green ONLINE).
5. **Cases:** Case creation, title editing, and detail viewing.
6. **Terminal:** Command execution and output inspection.
7. **Evidence:** CAS blob capture and SHA-256 address generation.
8. **Audit:** Verification of the cryptographic hash-chain provenance log.
9. **Browser:** Fail-closed Tor boundary verification.
10. **Reports:** Forensic report generation, atomic disk export, and cryptographic seal verification.
11. **Lifecycle:** Clean shutdown and reboot.

---

## Architecture & Security Principles

```
┌─────────────────────────────────────────────────────────────┐
│             PURSUE OS Desktop Shell (egui / Sway)           │
│     (Unprivileged UI — runs as pursue-investigator UID 1001)│
└───────────────────────────────┬─────────────────────────────┘
                                │ Local IPC (AF_UNIX socket)
                                │ /run/pursue/ipc.sock (0770)
┌───────────────────────────────▼─────────────────────────────┐
│          PURSUE Core Runtime Daemon (pursue-desktop)        │
│          (System Service — runs as pursue UID 990)          │
├────────────────┬──────────────┬──────────────┬──────────────┤
│  Case Service  │ Report Serv  │ Terminal Svc │ Browser Svc  │
├────────────────┴──────────────┴──────────────┴──────────────┤
│            Content-Addressed Evidence Store (CAS)           │
│        SHA-256 Immutable Blobs + Hash-Chained Audit         │
└─────────────────────────────────────────────────────────────┘
```

1. **Strict Privilege Separation (Decision A-008):** The desktop UI runs under the investigator account (`pursue-investigator`) with zero direct access to case files, child processes, or evidence storage. All operations must pass through IPC.
2. **Monolithic Binary Architecture (Decision A-011):** A single compiled Rust binary (`pursue-desktop`) operates in headless daemon mode under systemd (`pursue-runtime.service`) or in graphical shell mode under Sway.
3. **Fail-Closed Evidence Integrity:** Evidence blobs are immutable and addressed by SHA-256 hash. If an artifact is modified, missing, or mismatched, the case loader fails closed with an `IntegrityViolation`.
4. **Tamper-Evident Audit Trails:** Every case action produces a cryptographic event with `sha256(prev_hash || payload)`. Broken links or mutated history are immediately detected.
5. **Fail-Closed Tor Routing:** Tor routing is strictly fail-closed. If the SOCKS5 proxy or Tor daemon is unavailable, network requests are blocked. Direct internet fallback is prohibited.
6. **Defense-in-Depth Systemd Sandboxing:** The backend daemon enforces `ProtectSystem=strict`, `ProtectHome=true`, `PrivateTmp=true`, `ProtectClock=true`, `ProtectHostname=true`, `RestrictSUIDSGID=true`, `CapabilityBoundingSet=`, `DevicePolicy=closed`, and `RestrictAddressFamilies=AF_UNIX AF_INET AF_INET6`.

---

## Development Status & Roadmap

| Milestone | Stage | Status |
|---|---|---|
| **Phase 1A** | Architecture & Technical Locks | ✅ Complete |
| **Phase 1B** | Bootable Base Infrastructure | ✅ Complete |
| **Phase 1C** | Core Runtime & IPC Framework | ✅ Complete |
| **Phase 1D** | Case & Evidence Storage Foundation | ✅ Complete |
| **Phase 1E** | Investigation Terminal Foundation | ✅ Complete |
| **Phase 1F** | Investigation Browser & Tor Foundation | ✅ Complete |
| **Phase 2** | Desktop Shell & IPC Integration | ✅ Complete |
| **Phase 3** | Investigation Workspace & Case Lifecycle | ✅ Complete |
| **Phase 4** | Forensic Reporting & Cryptographic Export | ✅ Complete |
| **Phase 5** | Full OS Integration & Service Unification | ✅ Complete |
| **Phase 6 / Milestone 7** | Native Linux ISO Build & Boot Validation | ✅ Complete |
| **Milestone 8** | V1 Beta Hardening & Adversarial Validation | ✅ Complete |
| **Phase 9** | Product Integration & First-Boot Readiness | ✅ Complete |
| **Phase 10** | V1 Beta Release Candidate & QEMU Validation | ✅ Complete |
| **Current Stage** | **Owner Manual & Hardware Testing** | 🔄 **IN PROGRESS** |
| **Next Stage** | V1 Beta Stabilization & Bug Fixing | ⏳ Pending Test Results |
| **Future Stage** | Advanced Graph UI, Hardware Installer, Plugins | 📋 Planned |

---

## Repository Structure

```text
Pursue-OS/
├── build/                      # OS build manifests and system configurations
│   ├── config/                 # Systemd units, sysusers, tmpfiles, Sway configs
│   ├── debian/                 # Debian package lists
│   └── scripts/                # Rootfs builder, ISO assembler, config validator
├── crates/                     # Pure Rust modular workspace crates
│   ├── pursue-core/            # Foundational primitives, errors, identifiers
│   ├── pursue-evidence/        # Content-addressed storage, hashing, manifests
│   ├── pursue-case/            # Case containers, audit chains, file store
│   ├── pursue-runtime/         # System configuration, logging, IPC router
│   ├── pursue-terminal/        # Terminal sessions, process execution, capture
│   ├── pursue-browser/         # Browser routing, Tor boundaries, web capture
│   ├── pursue-report/          # Deterministic JSON/HTML reports, sealing
│   └── pursue-desktop/         # Monolithic egui shell & headless daemon
├── docs/                       # Authoritative technical documentation
│   ├── architecture/           # Security boundaries, IPC contracts
│   ├── core/                   # Master spec, decision record, product vision
│   ├── development/            # Implementation milestone records
│   └── release/                # Release plan, hardware audit, test checklists
└── tests/                      # Automated QEMU boot and live validation harness
```

---

## Building from Source

### Prerequisites
- Linux x86_64 host (Debian/Ubuntu or WSL2)
- Rust 1.85+ (stable toolchain)
- Dependencies: `xorriso`, `grub-pc-bin`, `grub-efi-amd64-bin`, `mksquashfs`, `debootstrap`, `qemu-system-x86_64`

### Build Commands
```bash
# 1. Run all workspace unit tests
cargo test --workspace

# 2. Build the monolithic static binary for Linux musl
cargo build --release --target x86_64-unknown-linux-musl --bin pursue-desktop

# 3. Validate build configuration files
bash build/scripts/validate-build-config.sh

# 4. Build the hybrid bootable ISO (requires root for debootstrap)
sudo bash build/scripts/build-base.sh target/pursue-rootfs
sudo bash build/scripts/build-iso.sh target/pursue-rootfs

# 5. Run automated QEMU cold-boot verification
python3 tests/qemu_verify_boot.py
```

---

## License & Legal

PURSUE OS is free and open-source software licensed under the **Apache License 2.0**.

See [`LICENSE`](LICENSE) and [`NOTICE`](NOTICE) for details.

---

### Project Status Summary
**PURSUE OS V1 Beta — Core Development Complete. Ready for Owner Manual Testing.**

---

<!-- SUPPORT-BLOCK:START -->
<div align="center">

### Support my work

I make these tools on my own and keep them free. If one helped you, you can chip in by UPI. Any amount.

<a href="https://p4inz-code.github.io/donate/"><img src="https://raw.githubusercontent.com/p4inz-code/donate/main/qr.svg" alt="UPI QR code. Scan it with any UPI app." width="200"></a>

`9321614988@jio`

On your phone? [Open the donation page](https://p4inz-code.github.io/donate/).

Outside India, or prefer a card?

<a href="https://buymeacoffee.com/p4inz"><img src="https://img.shields.io/badge/Buy%20me%20a%20coffee-p4inz-FFDD00?style=for-the-badge&logo=buymeacoffee&logoColor=black" alt="Buy me a coffee"></a>

</div>
<!-- SUPPORT-BLOCK:END -->

