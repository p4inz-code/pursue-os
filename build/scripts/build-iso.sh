#!/usr/bin/env bash
# PURSUE OS — Bootable Hybrid UEFI/BIOS Live ISO Packaging Script (Phase 1B + Phase 6)
# Converts a prepared PURSUE rootfs directory into a bootable ISO image.
#
# Requirements:
#   - Linux build host (Debian, Ubuntu, or WSL2 with root/sudo)
#   - mksquashfs, xorriso, grub-mkrescue / grub-efi-amd64-bin

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "${SCRIPT_DIR}/../.." && pwd)"

ROOTFS_DIR="${1:-${ROOT_DIR}/target/pursue-rootfs}"
ISO_OUTPUT="${2:-${ROOT_DIR}/target/pursue-os-v1-amd64.iso}"
WORK_DIR="${3:-/var/tmp/pursue-iso-work}"

cleanup() {
    if [[ -d "${WORK_DIR}" ]]; then
        rm -rf "${WORK_DIR}"
    fi
}
trap cleanup EXIT

echo "=========================================================="
echo " PURSUE OS — Building Hybrid Live ISO"
echo " Source rootfs: ${ROOTFS_DIR}"
echo " Target ISO:    ${ISO_OUTPUT}"
echo "=========================================================="

if [[ $EUID -ne 0 ]]; then
    echo "ERROR: Root privileges required for loopback mounting and ISO creation." >&2
    echo "Please execute: sudo $0 $@" >&2
    exit 1
fi

command -v mksquashfs >/dev/null 2>&1 || { echo "ERROR: mksquashfs not found." >&2; exit 1; }
command -v xorriso >/dev/null 2>&1 || { echo "ERROR: xorriso not found." >&2; exit 1; }

if [[ ! -d "${ROOTFS_DIR}" ]]; then
    echo "ERROR: Rootfs directory '${ROOTFS_DIR}' does not exist. Run build-base.sh first." >&2
    exit 1
fi

# Validate rootfs contains the PURSUE binary (Decision A-011)
if [[ ! -f "${ROOTFS_DIR}/usr/lib/pursue/bin/pursue-desktop" ]]; then
    echo "ERROR: pursue-desktop binary not found in rootfs at /usr/lib/pursue/bin/." >&2
    echo "Ensure build-base.sh installed the binary correctly." >&2
    exit 1
fi

rm -rf "${WORK_DIR}"
mkdir -p "${WORK_DIR}/live"
mkdir -p "${WORK_DIR}/boot/grub"

echo "[1/5] Creating compressed SquashFS image..."
mksquashfs "${ROOTFS_DIR}" "${WORK_DIR}/live/filesystem.squashfs" \
    -comp xz -b 1M -noappend

echo "[2/5] Extracting kernel and initramfs..."
VMLINUZ=$(find "${ROOTFS_DIR}/boot" -name 'vmlinuz*' | sort -V | tail -n 1)
INITRD=$(find "${ROOTFS_DIR}/boot" -name 'initrd.img*' | sort -V | tail -n 1)

if [[ -z "${VMLINUZ}" || -z "${INITRD}" ]]; then
    echo "ERROR: Kernel or initrd not found in rootfs /boot/." >&2
    echo "Ensure linux-image-amd64 is installed in the rootfs." >&2
    exit 1
fi

cp "${VMLINUZ}" "${WORK_DIR}/boot/vmlinuz"
cp "${INITRD}" "${WORK_DIR}/boot/initrd.img"

echo "[3/5] Generating GRUB EFI & BIOS boot configuration..."
cat << 'EOF' > "${WORK_DIR}/boot/grub/grub.cfg"
serial --speed=115200 --unit=0 --word=8 --parity=no --stop=1
terminal_input --append serial
terminal_output --append serial

set default=0
set timeout=5

menuentry "PURSUE OS — Forensic Investigation Workstation (Live RAM)" {
    linux /boot/vmlinuz boot=live components quiet splash security=apparmor console=ttyS0,115200 console=tty0
    initrd /boot/initrd.img
}

menuentry "PURSUE OS — Forensic Workstation (Serial Console Debug)" {
    linux /boot/vmlinuz boot=live components console=ttyS0,115200 console=tty0 systemd.journald.forward_to_console=1
    initrd /boot/initrd.img
}

menuentry "PURSUE OS — Safe Graphics / Failsafe Mode" {
    linux /boot/vmlinuz boot=live components nomodeset noapic console=ttyS0,115200 console=tty0
    initrd /boot/initrd.img
}
EOF

echo "[4/5] Assembling hybrid bootable ISO..."
mkdir -p "$(dirname "${ISO_OUTPUT}")"
if command -v grub-mkrescue >/dev/null 2>&1; then
    echo "Using grub-mkrescue for hybrid UEFI/BIOS bootable media..."
    grub-mkrescue -o "${ISO_OUTPUT}" "${WORK_DIR}" -- -volid "PURSUE_OS_V1"
else
    echo "Using xorriso mkisofs fallback..."
    xorriso -as mkisofs \
        -iso-level 3 \
        -full-iso9660-filenames \
        -volid "PURSUE_OS_V1" \
        -output "${ISO_OUTPUT}" \
        "${WORK_DIR}"
fi

echo "[5/5] Generating integrity checksum..."
sha256sum "${ISO_OUTPUT}" > "${ISO_OUTPUT}.sha256"

echo "=========================================================="
echo " ISO Generated successfully: ${ISO_OUTPUT}"
echo " SHA-256 Checksum: $(cat "${ISO_OUTPUT}.sha256")"
echo " Size: $(du -h "${ISO_OUTPUT}" | cut -f1)"
echo "=========================================================="
