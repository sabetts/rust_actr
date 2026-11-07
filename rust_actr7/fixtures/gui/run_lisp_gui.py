#!/usr/bin/env python3
"""Run the ignored GUI tests against a fresh Lisp dispatcher per case.

Each case gets a new SBCL process because the Lisp dispatcher retains the
client-owned command names after a connection closes.

Usage:
    python rust_actr7/fixtures/gui/run_lisp_gui.py
"""

import socket
import subprocess
import tempfile
import time
import uuid
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
ACTR = ROOT / "actr7.x"
CRATE = ROOT / "rust_actr7"
CURRENT_NET = ACTR / "environment" / "gui" / "init" / "05-current-net.tcl"
PORT = 2650
READY = "HARNESS-READY"


def home_value(name, default=None):
    try:
        return (Path.home() / name).read_text().strip()
    except OSError:
        return default


def port_open(host, port):
    try:
        with socket.create_connection((host, port), timeout=0.3):
            return True
    except OSError:
        return False


def stop(proc):
    if proc is None or proc.poll() is not None:
        return
    subprocess.run(
        ["taskkill", "/PID", str(proc.pid), "/T", "/F"],
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
    )
    try:
        proc.wait(timeout=5)
    except subprocess.TimeoutExpired:
        proc.kill()


def discover_tests():
    result = subprocess.run(
        [
            "cargo",
            "test",
            "--features",
            "env",
            "--lib",
            "lisp_",
            "--",
            "--list",
            "--ignored",
        ],
        cwd=CRATE,
        capture_output=True,
        text=True,
    )
    tests = []
    for line in result.stdout.splitlines():
        line = line.strip()
        if line.endswith(": test"):
            tests.append(line[: -len(": test")])
    return tests


def start_server():
    if port_open(home_value("act-r-address.txt", "127.0.0.1"), PORT) or port_open(
        "127.0.0.1", PORT
    ):
        raise RuntimeError(f"port {PORT} is already in use")

    ready = Path(tempfile.gettempdir()) / f"actr-gui-ready-{uuid.uuid4().hex}.txt"
    server_out = tempfile.TemporaryFile()
    server_err = tempfile.TemporaryFile()
    server = subprocess.Popen(
        [
            "sbcl",
            "--load",
            "load-act-r.lisp",
            "--eval",
            (
                '(progn (with-open-file (f "%s" :direction :output '
                ':if-exists :supersede :if-does-not-exist :create) '
                '(format f "%s") (finish-output f)) (loop (sleep 60)))'
            )
            % (ready.as_posix(), READY),
        ],
        cwd=ACTR,
        stdin=subprocess.PIPE,
        stdout=server_out,
        stderr=server_err,
    )

    deadline = time.time() + 180
    while time.time() < deadline:
        if server.poll() is not None:
            raise RuntimeError("SBCL server exited during startup")
        target = home_value("act-r-address.txt", "127.0.0.1")
        if ready.exists() and port_open(target, PORT):
            break
        time.sleep(0.1)
    else:
        stop(server)
        raise RuntimeError("SBCL server did not become ready")

    return {
        "proc": server,
        "ready": ready,
        "out": server_out,
        "err": server_err,
    }


def close_server(server):
    stop(server["proc"])
    time.sleep(0.2)
    server["out"].seek(0)
    server["err"].seek(0)
    stdout = server["out"].read().decode("utf-8", "replace")
    stderr = server["err"].read().decode("utf-8", "replace")
    server["out"].close()
    server["err"].close()
    try:
        server["ready"].unlink()
    except OSError:
        pass
    return stdout, stderr


def run_case(name):
    server = start_server()
    stdout = stderr = ""
    try:
        result = subprocess.run(
            [
                "cargo",
                "test",
                "--features",
                "env",
                "--lib",
                "--",
                "--exact",
                name,
                "--ignored",
                "--nocapture",
                "--test-threads=1",
            ],
            cwd=CRATE,
            capture_output=True,
            text=True,
        )
    finally:
        stdout, stderr = close_server(server)

    invalid_detail = "Invalid detail level" in stdout or "Invalid detail level" in stderr
    ok = result.returncode == 0 and not invalid_detail
    if not ok:
        print(f"\n[case output: {name}]")
        print(result.stdout[-6000:].rstrip())
        print(result.stderr[-6000:].rstrip())
        print(f"\n[server stdout tail: {name}]\n{stdout[-4000:].rstrip()}")
        print(f"\n[server stderr tail: {name}]\n{stderr[-4000:].rstrip()}")
        if invalid_detail:
            print("\nFAIL: Lisp reported an invalid detail level")
    return ok


def main():
    saved_net = CURRENT_NET.read_bytes() if CURRENT_NET.exists() else None
    try:
        tests = discover_tests()
        if not tests:
            raise RuntimeError("no ignored lisp_ GUI tests were discovered")

        failures = []
        for index, name in enumerate(tests, 1):
            if saved_net is not None:
                CURRENT_NET.write_bytes(saved_net)
            print(f"[{index}/{len(tests)}] {name}", flush=True)
            try:
                if not run_case(name):
                    failures.append(name)
            except Exception as error:  # noqa: BLE001 - report the case and continue
                print(f"FAIL {name}: {error}")
                failures.append(name)
            finally:
                if saved_net is not None:
                    CURRENT_NET.write_bytes(saved_net)

        print()
        if failures:
            print(f"FAILED {len(failures)}/{len(tests)}:")
            for name in failures:
                print(f"  - {name}")
            return 1
        print(f"PASS {len(tests)} Lisp GUI cases")
        return 0
    finally:
        if saved_net is not None:
            CURRENT_NET.write_bytes(saved_net)


if __name__ == "__main__":
    raise SystemExit(main())
