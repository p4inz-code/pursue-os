# PURSUE OS — V1 Beta Manual Boot Test Checklist

> **Purpose:** Step-by-step validation checklist for the owner's first manual boot of the PURSUE OS V1 Beta ISO on real or virtual hardware.
> **ISO Path:** `target/pursue-os-v1-amd64.iso` (Volume Label: `PURSUE_OS_V1`)
> **ISO Size:** 403,159,040 bytes (~385 MB)
> **SHA-256 Checksum:** `749263f12c4375ccb4c9079dc05b8c73c4586c9ba971422ebd19d8331776184a`
> **Architecture:** x86_64 (AMD64)
> **Boot Mode:** Hybrid BIOS + UEFI
> **First Physical Test Machine:** College PC (NVIDIA RTX 5060 / RTX 5070 Ti)

---

## Pre-Boot Requirements

- A USB drive (≥ 1 GB) or virtual machine capable of booting ISO images
- For USB: Flash with `dd`, Rufus (DD mode), or Ventoy
- For VM: Attach ISO as boot media, allocate ≥ 2 GB RAM, 2+ CPU cores
- **NVIDIA RTX Note:** On systems with very new NVIDIA GPUs (such as RTX 5060 / 5070 Ti), if the default boot entry fails modesetting, select `"PURSUE OS — Safe Graphics (NVIDIA / Software Fallback)"` from the GRUB menu.

---

## Boot Test Procedure

### Phase 1: Boot & Kernel

| # | Step | Expected Result | ✅/❌ |
|---|------|----------------|-------|
| 1 | Insert USB / attach ISO and power on | GRUB boot menu appears with normal and Safe Graphics options | |
| 2 | Wait for automatic boot (5 second timeout) or press Enter | Kernel begins loading | |
| 3 | Observe kernel messages on screen | `Linux pursue-os 6.12.x` kernel boots without panic | |


### Phase 2: Automatic Login & Desktop

| # | Step | Expected Result | ✅/❌ |
|---|------|----------------|-------|
| 4 | Wait for automatic login | `pursue-investigator` is logged in automatically (no password prompt) | |
| 5 | Sway compositor starts | Desktop environment appears (not a bare shell prompt) | |
| 6 | PURSUE Desktop application launches | "PURSUE OS — Forensic Investigation Shell" window appears | |
| 7 | Dashboard tab is visible | Dashboard renders with system information and IPC status | |
| 8 | IPC status indicator | Shows "ONLINE" (green) — the backend daemon is running | |

> **If you see a shell prompt instead of a desktop:** Sway may not have started. Type `sway` and press Enter. If Sway starts and PURSUE Desktop appears, the session startup is functional but the profile.d trigger may not have executed. Report this.

### Phase 3: Case Management

| # | Step | Expected Result | ✅/❌ |
|---|------|----------------|-------|
| 9 | Click "Cases" tab | Case management view appears | |
| 10 | Create a new case (enter an ID like `test-case-01` and a title) | Case is created successfully, status message confirms | |
| 11 | Case appears in the case list | Case ID and title are visible | |
| 12 | Click on the case to open it | Case details load (title, status, notes, evidence list) | |

### Phase 4: Terminal & Command Execution

| # | Step | Expected Result | ✅/❌ |
|---|------|----------------|-------|
| 13 | Click "Terminal" tab | Terminal console view appears | |
| 14 | Start a new terminal session | Session is created (linked to the active case) | |
| 15 | Execute a safe command (e.g., `uname -a` or `hostname`) | Command output appears showing Linux kernel information | |
| 16 | Output is displayed cleanly | No garbled text, no error messages | |

### Phase 5: Evidence Capture

| # | Step | Expected Result | ✅/❌ |
|---|------|----------------|-------|
| 17 | Capture evidence from the terminal output | Evidence is captured with a SHA-256 content address | |
| 18 | Click "Evidence" tab | Evidence list shows the captured artifact | |

### Phase 6: Audit Trail

| # | Step | Expected Result | ✅/❌ |
|---|------|----------------|-------|
| 19 | Click "Audit Trail" tab | Audit log shows hash-chained events (case creation, evidence capture) | |
| 20 | Verify audit chain integrity | Chain verification shows "verified: true" | |

### Phase 7: Browser & Network

| # | Step | Expected Result | ✅/❌ |
|---|------|----------------|-------|
| 21 | Click "Browser & Tor" tab | Browser console view appears | |
| 22 | Observe Tor/network status | Tor service status is displayed | |
| 23 | Verify `.onion` URLs are rejected in Direct mode | Navigation to `.onion` addresses fails with security boundary message | |

### Phase 8: Reports

| # | Step | Expected Result | ✅/❌ |
|---|------|----------------|-------|
| 24 | Click "Reports" tab | Report generation view appears | |
| 25 | Generate a report for the active case | Report generates successfully with SHA-256 hash | |
| 26 | Export report to disk | Report file is written, path is displayed | |
| 27 | Verify the exported report | Verification confirms cryptographic seal is intact | |

### Phase 9: Case Closure

| # | Step | Expected Result | ✅/❌ |
|---|------|----------------|-------|
| 28 | Return to "Cases" tab | Case list is visible | |
| 29 | Close the test case | Case status changes to "closed" | |

### Phase 10: System Lifecycle

| # | Step | Expected Result | ✅/❌ |
|---|------|----------------|-------|
| 30 | Shutdown the system (via terminal: `sudo poweroff`) | System shuts down cleanly without errors | |
| 31 | Reboot the system (via terminal: `sudo reboot`) | System reboots and returns to the desktop | |

---

## Expected First-Boot Result

**On a successful boot, you should see:**

1. GRUB boot menu appears → auto-selects after 5 seconds
2. Kernel boots (Linux 6.12.x messages scroll)
3. `pursue-investigator` is logged in automatically
4. Sway desktop compositor starts
5. PURSUE OS Desktop window appears with the Dashboard tab
6. IPC status shows ONLINE (green)
7. All tabs (Cases, Evidence, Timeline, Audit Trail, Terminal, Browser & Tor, Reports, Settings & IPC) are accessible
8. Creating a case, running a command, and capturing evidence works end-to-end

**What constitutes a REAL FAILURE:**

- Kernel panic or boot hang
- Login prompt requiring a password (autologin not working)
- Black screen with no desktop (Sway failed to start)
- PURSUE Desktop crashes immediately on startup
- IPC permanently shows OFFLINE
- Cannot create a case (permission errors)
- Cannot execute a terminal command
- Cannot capture evidence
- System hangs on shutdown

**What is NOT a failure:**

- `systemctl is-system-running` shows "degraded" — some optional system services may not start in a live ISO environment (this is expected and does not affect PURSUE functionality)
- Minor cosmetic differences in font rendering
- Sway showing informational messages in the background
- Network-dependent services being unavailable (e.g., if no network cable is connected)

---

## Reporting Results

After completing the test, note:

1. Which steps passed and which failed (use the ✅/❌ column)
2. Any error messages shown on screen
3. Any unexpected behavior not covered by this checklist
4. The hardware/VM configuration used for testing

---

## Technical Reference

| Component | Path |
|-----------|------|
| PURSUE binary | `/usr/lib/pursue/bin/pursue-desktop` |
| Configuration | `/etc/pursue/config.toml` |
| Runtime daemon | `pursue-runtime.service` (runs as `pursue` user) |
| IPC socket | `/run/pursue/ipc.sock` (mode 0770) |
| Case storage | `/var/lib/pursue/cases/` |
| Report storage | `/var/lib/pursue/reports/` |
| Sway config | `/etc/sway/config.d/00-pursue.conf` |
| Autologin | `/etc/systemd/system/getty@tty1.service.d/autologin.conf` |
| Session launcher | `/etc/profile.d/00-pursue-session.sh` |
