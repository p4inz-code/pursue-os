import subprocess
import socket
import time
import os
import sys

ISO_PATH = "/mnt/c/Users/Admin/Desktop/Pursue-OS/target/pursue-os-v1-amd64.iso"
SOCK_PATH = "/tmp/qemu-test-serial.sock"

if os.path.exists(SOCK_PATH):
    os.remove(SOCK_PATH)

print("[1] Starting QEMU with PURSUE OS ISO...")
qemu_cmd = [
    "qemu-system-x86_64",
    "-m", "2048",
    "-smp", "2",
    "-cdrom", ISO_PATH,
    "-boot", "d",
    "-nographic",
    "-serial", f"unix:{SOCK_PATH},server,nowait",
    "-nic", "none"
]

proc = subprocess.Popen(qemu_cmd, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)

try:
    for _ in range(50):
        if os.path.exists(SOCK_PATH):
            break
        time.sleep(0.1)

    print("[2] Connecting to QEMU serial socket...")
    s = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
    for _ in range(50):
        try:
            s.connect(SOCK_PATH)
            break
        except Exception:
            time.sleep(0.2)

    s.settimeout(1.0)

    def read_until(pattern, timeout=150):
        buf = ""
        start = time.time()
        while time.time() - start < timeout:
            try:
                data = s.recv(4096)
                if data:
                    text = data.decode("utf-8", errors="replace")
                    buf += text
                    print(text, end="", flush=True)
                    if pattern in buf:
                        return buf
            except socket.timeout:
                pass
        raise TimeoutError(f"Timed out waiting for '{pattern}' after {timeout}s")

    def send_line(line):
        s.sendall((line + "\n").encode("utf-8"))
        time.sleep(0.5)

    print("[3] Waiting for login prompt...")
    read_until("pursue-os login:", timeout=150)

    print("\n[4] Logging in as pursue-investigator...")
    send_line("pursue-investigator")
    read_until("pursue-investigator@pursue-os:", timeout=30)

    print("\n=== SYSTEM VERIFICATION START ===")

    def run_cmd(cmd, expect_str=None, timeout=30):
        print(f"\n>>> RUNNING: {cmd}")
        send_line(cmd)
        out = read_until("pursue-investigator@pursue-os:", timeout=timeout)
        if expect_str and expect_str not in out:
            raise AssertionError(f"Expected '{expect_str}' in output of '{cmd}'")
        return out

    # 1. System info
    run_cmd("uname -a", "Linux pursue-os")

    # 2. Systemd running status
    run_cmd("systemctl is-system-running")

    # 3. In-process service status (Decision A-011)
    run_cmd("systemctl status pursue-runtime.service", "active (running)")

    # 4. IPC Socket and permissions
    run_cmd("ls -la /run/pursue/ipc.sock", "srwxrwx--- 1 pursue pursue-investigator")

    # 5. User accounts and security boundaries
    run_cmd("id pursue", "uid=990(pursue)")
    run_cmd("id pursue-investigator", "uid=1001(pursue-investigator)")
    run_cmd("grep pursue /etc/passwd", "/usr/sbin/nologin")

    # 6. Storage directories and permissions
    run_cmd("ls -ld /var/lib/pursue/cases /var/lib/pursue/profiles /var/lib/pursue/reports /var/log/pursue", "cases")

    # 7. Tor service status
    run_cmd("systemctl status tor.service")

    # 8. Headless CLI execution test
    run_cmd("/usr/lib/pursue/bin/pursue-desktop --headless --config /etc/pursue/config.toml", "PURSUE OS Runtime — Headless Service Mode")

    # 9. Live Investigation Flow Test (Part 17)
    run_cmd("/usr/lib/pursue/bin/pursue-desktop --verify-live --config /etc/pursue/config.toml", "LIVE INVESTIGATION FLOW TEST: PASSED", timeout=40)

    # 10. Inspect exported report
    run_cmd("ls -la /var/lib/pursue/reports/")
    run_cmd("grep report_hash /var/lib/pursue/reports/live-report-*.json", "report_hash")

    print("\n=======================================================")
    print(" ALL REAL QEMU BOOT VALIDATION TESTS PASSED!")
    print("=======================================================")

finally:
    print("\nTerminating QEMU process...")
    proc.terminate()
    try:
        proc.wait(timeout=5)
    except Exception:
        proc.kill()
    if os.path.exists(SOCK_PATH):
        os.remove(SOCK_PATH)
