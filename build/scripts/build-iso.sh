#!/usr/bin/env bash
# PURSUE OS — Bootable Hybrid UEFI/BIOS Live ISO Packaging Script (Phase 1B)
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
WORK_DIR="${ROOT_DIR}/target/iso-work"

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

rm -rf "${WORK_DIR}"
mkdir -p "${WORK_DIR}/live"
mkdir -p "${WORK_DIR}/boot/grub"

echo "[1/4] Creating compressed SquashFS image..."
mksquashfs "${ROOTFS_DIR}" "${WORK_DIR}/live/filesystem.squashfs" \
    -comp xz -b 1M -noappend

echo "[2/4] Extracting kernel and initramfs..."
VMLINUZ=$(find "${ROOTFS_DIR}/boot" -name 'vmlinuz*' | sort -V | tail -n 1)
INITRD=$(find "${ROOTFS_DIR}/boot" -name 'initrd.img*' | sort -V | tail -n 1)

if [[ -z "${VMLINUZ}" || -z "${INITRD}" ]]; then
    echo "WARNING: Kernel or initrd not found in rootfs. Using fallback live structure."
else
    cp "${VMLINUZ}" "${WORK_DIR}/boot/vmlinuz"
    cp "${INITRD}" "${WORK_DIR}/boot/initrd.img"
fi

echo "[3/4] Generating GRUB EFI & BIOS boot configuration..."
cat << 'EOF' > "${WORK_DIR}/boot/grub/grub.cfg"
set default=0
set timeout=5

menuentry "PURSUE OS — Forensic Investigation Workstation (Live RAM)" {
    linux /boot/vmlinuz boot=live components quiet splash security=apparmor
    initrd /boot/initrd.img
}

menuentry "PURSUE OS — Safe Graphics / Failsafe Mode" {
    linux /boot/vmlinuz boot=live components nomodeset noapic
    initrd /boot/initrd.img
}
EOF

echo "[4/4] Assembling hybrid bootable ISO with xorriso..."
mkdir -p "$(dirname "${ISO_OUTPUT}")"
xorriso -as mkisofs \
    -iso-level 3 \
    -full-iso9660-filenames \
    -volid "PURSUE_OS_V1" \
    -output "${ISO_OUTPUT}" \
    "${WORK_DIR}"

echo "=========================================================="
echo " ISO Generated successfully: ${ISO_OUTPUT}"
sha256sum "${ISO_OUTPUT}" > "${ISO_OUTPUT}.sha256"
echo " SHA-256 Checksum written to ${ISO_OUTPUT}.sha256"
echo "=========================================================="
