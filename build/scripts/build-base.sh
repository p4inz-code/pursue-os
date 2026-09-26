#!/usr/bin/env bash
# PURSUE OS — Minimal Bootable Base Construction Script (Phase 1B)
# Builds a clean Debian base rootfs using debootstrap and installs PURSUE services.
#
# Requirements:
#   - Linux build host (Debian, Ubuntu, or WSL2 with root/sudo)
#   - debootstrap, systemd-container, binfmt-support

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "${SCRIPT_DIR}/../.." && pwd)"
BUILD_DIR="${ROOT_DIR}/build"

DEBIAN_RELEASE="${DEBIAN_RELEASE:-trixie}"
MIRROR_URL="${MIRROR_URL:-http://deb.debian.org/debian}"
TARGET_DIR="${1:-${ROOT_DIR}/target/pursue-rootfs}"

echo "=========================================================="
echo " PURSUE OS — Building Minimal Bootable Base (${DEBIAN_RELEASE})"
echo " Target directory: ${TARGET_DIR}"
echo "=========================================================="

# 1. Validation check
if [[ $EUID -ne 0 ]]; then
    echo "ERROR: Root privileges required for debootstrap rootfs assembly." >&2
    echo "Please execute: sudo $0 $@" >&2
    exit 1
fi

command -v debootstrap >/dev/null 2>&1 || {
    echo "ERROR: 'debootstrap' is not installed on this host." >&2
    exit 1
}

# 2. Clean previous build target if present
if [[ -d "${TARGET_DIR}" ]]; then
    echo "Cleaning existing target: ${TARGET_DIR}"
    rm -rf "${TARGET_DIR}"
fi
mkdir -p "${TARGET_DIR}"

# 3. Read package manifest
PACKAGE_LIST=$(grep -v '^#' "${BUILD_DIR}/debian/packages.list" | grep -v '^$' | tr '\n' ',' | sed 's/,$//')

echo "[1/5] Running debootstrap for ${DEBIAN_RELEASE}..."
debootstrap --variant=minbase \
    --include="${PACKAGE_LIST}" \
    "${DEBIAN_RELEASE}" \
    "${TARGET_DIR}" \
    "${MIRROR_URL}"

echo "[2/5] Installing PURSUE directory tree & permissions..."
mkdir -p "${TARGET_DIR}/etc/pursue"
mkdir -p "${TARGET_DIR}/var/lib/pursue/cases"
mkdir -p "${TARGET_DIR}/var/lib/pursue/profiles"
mkdir -p "${TARGET_DIR}/var/log/pursue"
mkdir -p "${TARGET_DIR}/run/pursue"
mkdir -p "${TARGET_DIR}/usr/lib/pursue/bin"
mkdir -p "${TARGET_DIR}/usr/lib/systemd/system"
mkdir -p "${TARGET_DIR}/usr/lib/systemd/system-preset"
mkdir -p "${TARGET_DIR}/usr/lib/sysusers.d"
mkdir -p "${TARGET_DIR}/usr/lib/tmpfiles.d"

echo "[3/5] Deploying configuration & systemd services..."
cp "${BUILD_DIR}/config/pursue-config.toml" "${TARGET_DIR}/etc/pursue/config.toml"
cp "${BUILD_DIR}/config/sysusers.d-pursue.conf" "${TARGET_DIR}/usr/lib/sysusers.d/pursue.conf"
cp "${BUILD_DIR}/config/tmpfiles.d-pursue.conf" "${TARGET_DIR}/usr/lib/tmpfiles.d/pursue.conf"
cp "${BUILD_DIR}/systemd/"*.service "${TARGET_DIR}/usr/lib/systemd/system/"
cp "${BUILD_DIR}/systemd/pursue.preset" "${TARGET_DIR}/usr/lib/systemd/system-preset/90-pursue.preset"

echo "[4/5] Copying compiled PURSUE release binaries..."
if [[ -f "${ROOT_DIR}/target/release/pursue-desktop" ]]; then
    cp "${ROOT_DIR}/target/release/pursue-desktop" "${TARGET_DIR}/usr/bin/pursue-desktop"
    chmod 0755 "${TARGET_DIR}/usr/bin/pursue-desktop"
fi

echo "[5/5] Configuring hostname & systemd units inside rootfs..."
echo "pursue-os" > "${TARGET_DIR}/etc/hostname"
echo "127.0.0.1 localhost pursue-os" > "${TARGET_DIR}/etc/hosts"

chroot "${TARGET_DIR}" systemd-sysusers || true
chroot "${TARGET_DIR}" systemd-tmpfiles --create || true
chroot "${TARGET_DIR}" systemctl preset-all || true

echo "=========================================================="
echo " Minimal Bootable Base assembly complete: ${TARGET_DIR}"
echo "=========================================================="
