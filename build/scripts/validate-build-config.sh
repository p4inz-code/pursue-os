#!/usr/bin/env bash
# PURSUE OS — Build Configuration & Layout Validator (Phase 1B)
# Validates build manifests, systemd unit files, sysusers, and tmpfiles specifications.
# Cross-platform: runs on Linux, macOS, and Git Bash on Windows.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "${SCRIPT_DIR}/../.." && pwd)"
BUILD_DIR="${ROOT_DIR}/build"

echo "=== PURSUE OS Build Configuration Validation ==="

FAILURES=0

check_file() {
    local path="$1"
    local desc="$2"
    if [[ -f "$path" ]]; then
        echo "  [OK] Found $desc: $(basename "$path")"
    else
        echo "  [FAIL] Missing $desc: $path" >&2
        FAILURES=$((FAILURES + 1))
    fi
}

echo "1. Checking build files presence:"
check_file "${BUILD_DIR}/debian/packages.list" "Debian package manifest"
check_file "${BUILD_DIR}/systemd/pursue-runtime.service" "pursue-runtime unit"
check_file "${BUILD_DIR}/systemd/pursue-terminal.service" "pursue-terminal unit"
check_file "${BUILD_DIR}/systemd/pursue-browser.service" "pursue-browser unit"
check_file "${BUILD_DIR}/systemd/pursue-tor.service" "pursue-tor unit"
check_file "${BUILD_DIR}/systemd/pursue.preset" "systemd preset file"
check_file "${BUILD_DIR}/config/sysusers.d-pursue.conf" "sysusers configuration"
check_file "${BUILD_DIR}/config/tmpfiles.d-pursue.conf" "tmpfiles layout definition"
check_file "${BUILD_DIR}/config/pursue-config.toml" "system configuration template"
check_file "${BUILD_DIR}/scripts/build-base.sh" "build-base script"
check_file "${BUILD_DIR}/scripts/build-iso.sh" "build-iso script"

echo "2. Validating systemd unit file structure:"
for unit in "${BUILD_DIR}/systemd/"*.service; do
    name=$(basename "$unit")
    if grep -q "\[Unit\]" "$unit" && grep -q "\[Service\]" "$unit" && grep -q "\[Install\]" "$unit"; then
        echo "  [OK] Valid systemd sections in $name"
    else
        echo "  [FAIL] Missing standard sections in $name" >&2
        FAILURES=$((FAILURES + 1))
    fi
done

echo "3. Validating security defaults in systemd units:"
if grep -q "ProtectSystem=" "${BUILD_DIR}/systemd/pursue-runtime.service" && \
   grep -q "NoNewPrivileges=true" "${BUILD_DIR}/systemd/pursue-runtime.service"; then
    echo "  [OK] Hardening flags present in pursue-runtime.service"
else
    echo "  [FAIL] Missing hardening flags in pursue-runtime.service" >&2
    FAILURES=$((FAILURES + 1))
fi

echo "4. Validating package list integrity:"
PKG_COUNT=$(grep -v '^#' "${BUILD_DIR}/debian/packages.list" | grep -v '^$' | wc -l)
if [[ $PKG_COUNT -ge 20 ]]; then
    echo "  [OK] Package manifest contains $PKG_COUNT packages"
else
    echo "  [FAIL] Package manifest has suspiciously few entries ($PKG_COUNT)" >&2
    FAILURES=$((FAILURES + 1))
fi

echo "5. Validating sysusers and tmpfiles rules:"
if grep -q "pursue-investigator" "${BUILD_DIR}/config/sysusers.d-pursue.conf" && \
   grep -q "pursue" "${BUILD_DIR}/config/sysusers.d-pursue.conf"; then
    echo "  [OK] sysusers defines pursue and pursue-investigator"
else
    echo "  [FAIL] sysusers missing required users/groups" >&2
    FAILURES=$((FAILURES + 1))
fi

if grep -q "/run/pursue" "${BUILD_DIR}/config/tmpfiles.d-pursue.conf" && \
   grep -q "/var/lib/pursue/cases" "${BUILD_DIR}/config/tmpfiles.d-pursue.conf"; then
    echo "  [OK] tmpfiles defines /run/pursue and /var/lib/pursue/cases"
else
    echo "  [FAIL] tmpfiles missing core directories" >&2
    FAILURES=$((FAILURES + 1))
fi

echo "==============================================="
if [[ $FAILURES -eq 0 ]]; then
    echo " BUILD CONFIGURATION VALIDATION PASSED (0 errors)"
    exit 0
else
    echo " BUILD CONFIGURATION VALIDATION FAILED ($FAILURES errors)" >&2
    exit 1
fi
