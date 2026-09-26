# PURSUE OS — Minimal Bootable Base Specification (Phase 1B)

> **Category:** B — Implementation documentation (maintained with the code).  
> **Status:** Authoritative specification for Phase 1B; implementation configurations in `build/`.  
> **Scope:** Phase 1B Minimal Bootable Base: Debian base configuration, systemd init architecture, directory layout, privilege separation, systemd unit definitions, user/group models, network policies, and reproducible image build pipeline.

---

## 1. Purpose & Identity

Phase 1B establishes the minimal bootable operating system foundation for PURSUE OS.
In accordance with `docs/core/DECISION_RECORD.md` §B7 and `docs/core/PURSUE_V1_CONTRACT.md` §1:
- Base Distribution: **Debian 13 (Trixie) / Debian 12 (Bookworm) minimal base**.
- Initialization & Supervision: **systemd**.
- Security Profile: Hardened, investigator-controlled, privilege-separated.
- Explicit Non-Goal: Not a Kali clone, not a generic penetration testing distro, not a cyberpunk toy. It is a purpose-built investigator workstation.

---

## 2. Filesystem Layout & Storage Boundaries

PURSUE OS establishes strict filesystem conventions across all packages and services:

| Path | Purpose | Permissions | Owner:Group |
| :--- | :--- | :--- | :--- |
| `/etc/pursue/` | Platform configuration files (`config.toml`) | `0755` | `root:root` |
| `/etc/pursue/services.d/` | Service descriptor drop-ins | `0755` | `root:root` |
| `/var/lib/pursue/` | State and case repository root | `0750` | `pursue:pursue-investigator` |
| `/var/lib/pursue/cases/` | Primary forensic case containers | `0770` | `pursue:pursue-investigator` |
| `/var/lib/pursue/profiles/` | Isolated browser session profiles | `0700` | `investigator:pursue-investigator` |
| `/var/log/pursue/` | Structured JSON log journal | `0750` | `pursue:pursue-investigator` |
| `/run/pursue/` | Volatile runtime IPC sockets (`ipc.sock`) | `0770` | `pursue:pursue-investigator` |
| `/usr/lib/pursue/bin/` | PURSUE daemon and core binaries | `0755` | `root:root` |
| `/usr/bin/pursue-desktop` | Graphical investigator desktop entry point | `0755` | `root:root` |

---

## 3. Users, Groups & Privilege Separation

### Dedicated Users and Groups
1. **`pursue` (System Daemon User)**:
   - System user (UID < 1000), no login shell (`/usr/sbin/nologin`).
   - Runs `pursue-runtime` background daemons, supervising IPC router and logging sinks.
2. **`pursue-investigator` (Security Group)**:
   - Shared group controlling access to the volatile IPC socket (`/run/pursue/ipc.sock`) and case stores.
3. **`investigator` (Default Interactive User)**:
   - Standard unprivileged user account (UID 1000).
   - Member of `pursue-investigator`, `audio`, `video`, `network`.
   - **Critical Rule**: The desktop shell, terminal GUI, and browser NEVER run as root. They run as `investigator`.

---

## 4. Service Architecture & systemd Supervision

The platform services boot in strict order via systemd:

```
systemd (PID 1)
   │
   ├──> tor.service (SOCKS5 proxy on 127.0.0.1:9050, Control on 127.0.0.1:9051)
   │
   ├──> pursue-runtime.service (Core daemon, IPC socket /run/pursue/ipc.sock)
   │       │
   │       ├──> pursue-terminal.service (Terminal execution supervisor)
   │       └──> pursue-browser.service (Browser & Tor routing supervisor)
   │
   └──> display-manager.service / graphical.target
           │
           └──> pursue-desktop (User graphical shell under 'investigator' session)
                   │
                   └── (Connects to /run/pursue/ipc.sock via IPC client)
```

---

## 5. Network Policy & Firewall Baseline

- **Default Firewall**: `nftables` with default-drop ingress policy.
- **Tor Isolation**:
  - Outbound traffic in Tor sessions is directed to `127.0.0.1:9050`.
  - DNS leaks are prevented by forcing SOCKS5h remote hostname resolution.
- **Direct Networking**:
  - Direct HTTP/HTTPS traffic is allowed for the unprivileged `investigator` user when direct mode is selected.
  - No silent transparent interception: the investigator retains complete visibility over network routing.

---

## 6. Minimal Base Package Selection

Only essential packages are installed in the minimal base image:
- **Core OS**: `systemd`, `systemd-sysv`, `udev`, `dbus`, `linux-image-amd64`, `grub-efi-amd64-bin`, `grub-pc-bin`.
- **Networking & Tor**: `iproute2`, `nftables`, `tor`, `ca-certificates`, `wpasupplicant`.
- **Display & Audio Baseline**: `wayland`, `weston` or `sway` / lightweight display server, `pipewire`.
- **PURSUE Foundation**: `pursue-runtime`, `pursue-terminal`, `pursue-browser`, `pursue-desktop`.
- **Exclusions**: Zero hacking game packages, zero duplicate OSINT tools, zero telemetry daemons.

---

## 7. Reproducible Build Pipeline

The build infrastructure resides in `build/`:
- `build/debian/packages.list`: Exact debian package manifest.
- `build/systemd/`: systemd unit files (`pursue-runtime.service`, etc.).
- `build/config/`: Default `config.toml`, `sysusers.d`, `tmpfiles.d`.
- `build/scripts/build-base.sh`: Rootfs generation script via `debootstrap`.
- `build/scripts/build-iso.sh`: Hybrid live UEFI/BIOS ISO packaging script via `xorriso` and `mksquashfs`.
- `build/scripts/validate-build-config.sh`: Automated validator checking syntax and security boundaries.

### Linux Build Host Execution
On a dedicated Linux build machine (or WSL with root privileges):
```bash
sudo ./build/scripts/build-base.sh --output /var/tmp/pursue-rootfs
sudo ./build/scripts/build-iso.sh --rootfs /var/tmp/pursue-rootfs --output pursue-os-v1-amd64.iso
```
