#!/usr/bin/env bash
# PURSUE OS — Minimal Bootable Base Construction Script (Phase 1B + Phase 5)
# Builds a clean Debian base rootfs using debootstrap and installs PURSUE services.
#
# Architecture Decision A-011: single monolithic binary (pursue-desktop).
# All domain services (terminal, browser, case, report) run in-process.
#
# Requirements:
#   - Linux build host (Debian, Ubuntu, or WSL2 with root/sudo)
#   - debootstrap, systemd-container, binfmt-support
#   - Pre-built binary: target/x86_64-unknown-linux-gnu/release/pursue-desktop

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

# 1. Validation checks
if [[ $EUID -ne 0 ]]; then
    echo "ERROR: Root privileges required for debootstrap rootfs assembly." >&2
    echo "Please execute: sudo $0 $@" >&2
    exit 1
fi

command -v debootstrap >/dev/null 2>&1 || {
    echo "ERROR: 'debootstrap' is not installed on this host." >&2
    exit 1
}

# Verify the cross-compiled PURSUE binary exists
PURSUE_BINARY="${ROOT_DIR}/target/x86_64-unknown-linux-musl/release/pursue-desktop"
if [[ ! -f "${PURSUE_BINARY}" ]]; then
    PURSUE_BINARY="${ROOT_DIR}/target/x86_64-unknown-linux-gnu/release/pursue-desktop"
fi
if [[ ! -f "${PURSUE_BINARY}" ]]; then
    # Fall back to native release binary (for native Linux builds)
    PURSUE_BINARY="${ROOT_DIR}/target/release/pursue-desktop"
fi

if [[ ! -f "${PURSUE_BINARY}" ]]; then
    echo "ERROR: pursue-desktop binary not found." >&2
    echo "Run: cargo build --workspace --release --target x86_64-unknown-linux-gnu" >&2
    exit 1
fi

echo "Using PURSUE binary: ${PURSUE_BINARY}"

# 2. Clean previous build target if present
if [[ -d "${TARGET_DIR}" ]]; then
    echo "Cleaning existing target: ${TARGET_DIR}"
    rm -rf "${TARGET_DIR}"
fi
mkdir -p "${TARGET_DIR}"

# 3. Read package manifest
PACKAGE_LIST=$(grep -v '^#' "${BUILD_DIR}/debian/packages.list" | grep -v '^$' | tr '\n' ',' | sed 's/,$//')

echo "[1/6] Running debootstrap for ${DEBIAN_RELEASE}..."
debootstrap --variant=minbase \
    --include="${PACKAGE_LIST}" \
    "${DEBIAN_RELEASE}" \
    "${TARGET_DIR}" \
    "${MIRROR_URL}"

echo "[2/6] Installing PURSUE directory tree & permissions..."
mkdir -p "${TARGET_DIR}/etc/pursue"
mkdir -p "${TARGET_DIR}/var/lib/pursue/cases"
mkdir -p "${TARGET_DIR}/var/lib/pursue/profiles"
mkdir -p "${TARGET_DIR}/var/lib/pursue/reports"
mkdir -p "${TARGET_DIR}/var/log/pursue"
mkdir -p "${TARGET_DIR}/run/pursue"
mkdir -p "${TARGET_DIR}/usr/lib/pursue/bin"
mkdir -p "${TARGET_DIR}/usr/lib/systemd/system"
mkdir -p "${TARGET_DIR}/usr/lib/systemd/system-preset"
mkdir -p "${TARGET_DIR}/usr/lib/sysusers.d"
mkdir -p "${TARGET_DIR}/usr/lib/tmpfiles.d"

echo "[3/6] Deploying configuration & systemd services..."
cp "${BUILD_DIR}/config/pursue-config.toml" "${TARGET_DIR}/etc/pursue/config.toml"
cp "${BUILD_DIR}/config/sysusers.d-pursue.conf" "${TARGET_DIR}/usr/lib/sysusers.d/pursue.conf"
cp "${BUILD_DIR}/config/tmpfiles.d-pursue.conf" "${TARGET_DIR}/usr/lib/tmpfiles.d/pursue.conf"
cp "${BUILD_DIR}/systemd/"*.service "${TARGET_DIR}/usr/lib/systemd/system/"
cp "${BUILD_DIR}/systemd/pursue.preset" "${TARGET_DIR}/usr/lib/systemd/system-preset/90-pursue.preset"

# Sway compositor autostart & session integration
mkdir -p "${TARGET_DIR}/etc/sway/config.d"
cp "${BUILD_DIR}/config/sway-config.d-pursue.conf" "${TARGET_DIR}/etc/sway/config.d/00-pursue.conf"
mkdir -p "${TARGET_DIR}/etc/profile.d"
cp "${BUILD_DIR}/config/profile.d-pursue-session.sh" "${TARGET_DIR}/etc/profile.d/00-pursue-session.sh"
chmod 0755 "${TARGET_DIR}/etc/profile.d/00-pursue-session.sh"

# Console autologin for pursue-investigator on tty1
mkdir -p "${TARGET_DIR}/etc/systemd/system/getty@tty1.service.d"
cp "${BUILD_DIR}/config/getty-autologin.conf" "${TARGET_DIR}/etc/systemd/system/getty@tty1.service.d/autologin.conf"

echo "[4/6] Installing pursue-desktop binary (Decision A-011: single binary)..."
cp "${PURSUE_BINARY}" "${TARGET_DIR}/usr/lib/pursue/bin/pursue-desktop"
chmod 0755 "${TARGET_DIR}/usr/lib/pursue/bin/pursue-desktop"

echo "[5/6] Configuring hostname & systemd units inside rootfs..."
echo "pursue-os" > "${TARGET_DIR}/etc/hostname"
echo "127.0.0.1 localhost pursue-os" > "${TARGET_DIR}/etc/hosts"

chroot "${TARGET_DIR}" systemd-sysusers || true
chroot "${TARGET_DIR}" systemd-tmpfiles --create || true
chroot "${TARGET_DIR}" systemctl preset-all || true

echo "[6/6] Setting ownership & permissions..."
mkdir -p "${TARGET_DIR}/home/pursue-investigator"
chroot "${TARGET_DIR}" chown -R pursue-investigator:pursue-investigator /home/pursue-investigator || true
chroot "${TARGET_DIR}" usermod -aG sudo,video,audio,input pursue-investigator || true
chroot "${TARGET_DIR}" passwd -d pursue-investigator || true

# Runtime directories (owned by pursue system daemon)
chroot "${TARGET_DIR}" chown -R pursue:pursue-investigator /var/lib/pursue || true
chroot "${TARGET_DIR}" chmod -R 0770 /var/lib/pursue/cases || true
chroot "${TARGET_DIR}" chmod -R 0770 /var/lib/pursue/profiles || true
chroot "${TARGET_DIR}" chmod -R 0770 /var/lib/pursue/reports || true
chroot "${TARGET_DIR}" chown -R pursue:pursue-investigator /var/log/pursue || true
mkdir -p "${TARGET_DIR}/run/pursue"
chroot "${TARGET_DIR}" chown -R pursue:pursue-investigator /run/pursue || true
chroot "${TARGET_DIR}" chmod 0770 /run/pursue || true


echo "=========================================================="
echo " Minimal Bootable Base assembly complete: ${TARGET_DIR}"
echo "=========================================================="
