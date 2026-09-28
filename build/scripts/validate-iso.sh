#!/usr/bin/env bash
# PURSUE OS — ISO Self-Check Validation Script (Phase 6)
# Performs automated structural and integrity checks on a generated ISO.
#
# Usage: validate-iso.sh [path/to/iso]
#
# Checks:
#   1. ISO file exists and has minimum viable size
#   2. SHA-256 checksum file exists and matches
#   3. ISO contains expected live boot structure
#   4. SquashFS filesystem.squashfs is present
#   5. GRUB configuration is present
#   6. Kernel and initramfs are present

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "${SCRIPT_DIR}/../.." && pwd)"

ISO_PATH="${1:-${ROOT_DIR}/target/pursue-os-v1-amd64.iso}"
if [[ -f "${ISO_PATH}" ]]; then
    ISO_PATH="$(cd "$(dirname "${ISO_PATH}")" && pwd)/$(basename "${ISO_PATH}")"
fi
CHECKSUM_PATH="${ISO_PATH}.sha256"

echo "=== PURSUE OS ISO Self-Check Validation ==="
echo "ISO: ${ISO_PATH}"

FAILURES=0
CHECKS=0

check() {
    CHECKS=$((CHECKS + 1))
    local desc="$1"
    local result="$2"
    if [[ "$result" == "pass" ]]; then
        echo "  [OK] $desc"
    else
        echo "  [FAIL] $desc" >&2
        FAILURES=$((FAILURES + 1))
    fi
}

echo "1. File existence and size:"
if [[ -f "${ISO_PATH}" ]]; then
    check "ISO file exists" "pass"
    ISO_SIZE=$(stat -c%s "${ISO_PATH}" 2>/dev/null || stat -f%z "${ISO_PATH}" 2>/dev/null || echo 0)
    # Minimum viable ISO size: 50MB (bare Debian + kernel + PURSUE binary)
    if [[ ${ISO_SIZE} -ge 52428800 ]]; then
        check "ISO size is >= 50MB (${ISO_SIZE} bytes)" "pass"
    else
        check "ISO size is >= 50MB (only ${ISO_SIZE} bytes)" "fail"
    fi
else
    check "ISO file exists" "fail"
    echo "  Cannot proceed with structural checks without ISO file." >&2
    echo "======================================="
    echo " ISO SELF-CHECK FAILED (${FAILURES}/${CHECKS} checks failed)"
    exit 1
fi

echo "2. Checksum verification:"
if [[ -f "${CHECKSUM_PATH}" ]]; then
    check "SHA-256 checksum file exists" "pass"
    if sha256sum -c "${CHECKSUM_PATH}" >/dev/null 2>&1 || \
       (cd "$(dirname "${ISO_PATH}")" && sha256sum -c "${CHECKSUM_PATH}" >/dev/null 2>&1) || \
       (cd "$(dirname "${ISO_PATH}")" && sha256sum -c "$(basename "${CHECKSUM_PATH}")" >/dev/null 2>&1); then
        check "SHA-256 checksum matches" "pass"
    else
        check "SHA-256 checksum matches" "fail"
    fi
else
    check "SHA-256 checksum file exists" "fail"
fi

echo "3. ISO structure inspection:"
# Use xorriso to list ISO contents if available
if command -v xorriso >/dev/null 2>&1; then
    ISO_LISTING=$(xorriso -osirrox on -indev "${ISO_PATH}" -find / -type f 2>/dev/null || true)

    # Check for live boot structure
    if echo "${ISO_LISTING}" | grep -q "filesystem.squashfs"; then
        check "SquashFS filesystem present in /live/" "pass"
    else
        check "SquashFS filesystem present in /live/" "fail"
    fi

    if echo "${ISO_LISTING}" | grep -q "grub.cfg"; then
        check "GRUB configuration present" "pass"
    else
        check "GRUB configuration present" "fail"
    fi

    if echo "${ISO_LISTING}" | grep -q "vmlinuz"; then
        check "Kernel image (vmlinuz) present" "pass"
    else
        check "Kernel image (vmlinuz) present" "fail"
    fi

    if echo "${ISO_LISTING}" | grep -q "initrd"; then
        check "Initramfs (initrd.img) present" "pass"
    else
        check "Initramfs (initrd.img) present" "fail"
    fi
else
    echo "  [SKIP] xorriso not available — cannot inspect ISO structure"
fi

echo "4. Volume label verification:"
if command -v xorriso >/dev/null 2>&1; then
    VOL_ID=$(xorriso -indev "${ISO_PATH}" -pvd_info 2>/dev/null | grep "Volume Id" || true)
    if echo "${VOL_ID}" | grep -q "PURSUE_OS_V1"; then
        check "Volume label is PURSUE_OS_V1" "pass"
    else
        check "Volume label is PURSUE_OS_V1 (got: ${VOL_ID})" "fail"
    fi
else
    echo "  [SKIP] xorriso not available — cannot verify volume label"
fi

echo "======================================="
PASSED=$((CHECKS - FAILURES))
if [[ $FAILURES -eq 0 ]]; then
    echo " ISO SELF-CHECK PASSED (${PASSED}/${CHECKS} checks passed)"
    exit 0
else
    echo " ISO SELF-CHECK FAILED (${FAILURES}/${CHECKS} checks failed)" >&2
    exit 1
fi
