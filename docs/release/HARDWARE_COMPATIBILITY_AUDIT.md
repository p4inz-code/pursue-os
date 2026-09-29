# PURSUE OS — V1 Beta Hardware Compatibility Audit

> **Status:** Final Pre-Release Hardware Audit Complete  
> **Release Candidate ISO:** `target/pursue-os-v1-amd64.iso`  
> **ISO Size:** 403,159,040 bytes (~385 MB)  
> **SHA-256 Checksum:** `749263f12c4375ccb4c9079dc05b8c73c4586c9ba971422ebd19d8331776184a`  
> **Kernel Version:** Linux `6.12.107+deb13-amd64` (Debian 13 Trixie)  
> **Default User:** `pursue-investigator` (passwordless sudo, tty1 autologin)  

---

## 1. Supported Architecture vs. Physically Verified Hardware

| Category | Specification | Details / Verification Status |
|---|---|---|
| **Supported Target Architecture** | `x86_64` (AMD64) | Compliant with standard x86-64-v1 baseline; no host-specific CPU instructions required. |
| **Firmware Modes** | Hybrid UEFI & Legacy BIOS | ISO produced via `grub-mkrescue` with El Torito MBR + GPT EFI system partition. |
| **Boot Medium** | USB Live Flash Drive / Virtual Drive | ISO9660 + SquashFS + live-boot overlayfs in RAM. |
| **Physically Verified Hardware** | QEMU virtualized hardware | Verified clean boot, user login, systemd multi-user (`running`), IPC, and 7/7 live forensic flow. |
| **First Physical Test Target** | College PC with NVIDIA RTX 5060 / RTX 5070 Ti | **Pending Owner First Manual Boot Test**. Hardware hardening in place; physical validation pending. |

> [!IMPORTANT]
> **No Universal Guarantee:** We do NOT claim physical verification for hardware not yet physically tested. The system architecture has been hardened to maximize first-boot compatibility and graceful fallback across modern x86_64 PCs, especially for modern NVIDIA RTX systems.

---

## 2. GPU & Display Subsystem Compatibility

### Graphical Boot Path
```
Kernel Boot (vmlinuz-6.12)
  └── DRM / KMS (i915, amdgpu, nouveau, or simpledrm / efifb)
        └── Wayland Compositor (Sway 1.10.1 via wlroots 0.18)
              └── Desktop Shell (pursue-desktop via eframe / egui 0.36 + wgpu)
```

### NVIDIA Compatibility Hardening (RTX 50-Series: 5060, 5070, 5070 Ti, 5080, 5090)
1. **Sway `--unsupported-gpu` Flag:** Sway by default aborts when proprietary NVIDIA drivers are detected. The startup script in `/etc/profile.d/00-pursue-session.sh` invokes `sway --unsupported-gpu` so the display server does not refuse to start.
2. **Software Cursor Emulation (`WLR_NO_HARDWARE_CURSORS=1`):** NVIDIA and generic framebuffers frequently fail hardware cursor plane initialization. Software cursor rendering is enforced globally to prevent invisible mouse pointer issues.
3. **Software Renderer Fallback (`WLR_RENDERER_ALLOW_SOFTWARE=1`):** In wlroots 0.18, software rendering (Pixman / llvmpipe) is disabled by default. Enabling this variable ensures wlroots falls back gracefully rather than crashing if 3D acceleration is missing.
4. **Mesa Vulkan Lavapipe (`mesa-vulkan-drivers`):** The CPU-based Vulkan software rasterizer (`lavapipe` / `libvulkan_lvp.so`) is installed in the rootfs, ensuring `wgpu` in `pursue-desktop` has a functional Vulkan adapter even when hardware GPU acceleration is unavailable.
5. **GRUB Safe Graphics Option:** A dedicated boot entry is provided:
   ```
   menuentry "PURSUE OS — Safe Graphics (NVIDIA / Software Fallback)" {
       linux /boot/vmlinuz boot=live components modprobe.blacklist=nouveau nouveau.modeset=0 console=ttyS0,115200 console=tty0
       initrd /boot/initrd.img
   }
   ```
   If Nouveau in kernel 6.12 encounters issues with the newer Blackwell architecture, this entry blacklists Nouveau and boots to the UEFI framebuffer with clean software rendering.
6. **Graceful Graphical Recovery:** If Sway exits non-zero, the session does not loop or crash-restart getty. It drops cleanly to the bash prompt with diagnostic instructions and verifies that the core `pursue-runtime` daemon remains operational in the background.

---

## 3. CPU Compatibility

- **Baseline:** Standard 64-bit x86_64 (`x86-64-v1`).
- **Binary Target:** `x86_64-unknown-linux-musl` static-PIE executable.
- **Instruction Sets:** Requires only baseline x86_64 instructions (MMX, SSE, SSE2, CMPXCHG16B).
- **No Native Assumptions:** Builds do not use `-C target-cpu=native`. Compatible with all modern Intel (Core i3/i5/i7/i9, Xeon) and AMD (Ryzen 3/5/7/9, Threadripper, EPYC) processors.

---

## 4. Memory & Storage Specifications

| Resource | Minimum Required | Recommended for V1 Beta |
|---|---|---|
| **System RAM** | 2.0 GB | 4.0 GB or higher |
| **Boot Drive** | 1.0 GB USB Flash Drive | 4.0 GB+ USB 3.0 Flash Drive |
| **Storage Architecture** | RAM OverlayFS | Root changes remain in volatile RAM. |
| **Case Storage** | `/var/lib/pursue/cases` (tmpfs) | Fallback to `/tmp/pursue-cases-store` if unprivileged. |
| **Reports** | `/var/lib/pursue/reports` | Fallback to `/tmp` if unprivileged. |

---

## 5. Network & Peripheral Independence

- **No Network Dependency on Boot:** Ethernet and Wi-Fi are completely optional. The OS was verified booting to a `running` system state under QEMU with `-nic none`.
- **Tor Independence:** The runtime service treats Tor as `Wants=tor.service`, not `Requires=`. If no internet or network interface is present, the forensic workspace starts and remains fully operational offline.
- **Audio / Camera / Bluetooth:** Failure or absence of audio devices, webcams, or Bluetooth does not block boot or desktop initialization.

---

## 6. Real-Hardware Risk Mitigation Summary

| Risk | Mitigation |
|---|---|
| NVIDIA proprietary driver blocking Sway | `sway --unsupported-gpu` bypass flag in `profile.d` |
| Blackwell RTX 50-series Nouveau crash | `modprobe.blacklist=nouveau nouveau.modeset=0` GRUB entry |
| Missing hardware cursor planes | `WLR_NO_HARDWARE_CURSORS=1` globally exported |
| Missing hardware 3D acceleration | `WLR_RENDERER_ALLOW_SOFTWARE=1` + `mesa-vulkan-drivers` (lavapipe) |
| Display server failure causing agetty crash loop | Diagnostic trap in `profile.d` with bash recovery prompt |
| NVMe / SATA drive discovery failure | `ahci.ko.xz`, `nvme.ko.xz`, `usb-storage.ko.xz`, `uas.ko.xz` in initrd |
| No network card present | Systemd units configured with `Wants=` for graceful offline boot |
