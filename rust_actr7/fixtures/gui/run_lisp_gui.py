#!/usr/bin/env python3
"""Run the ignored GUI tests against a fresh Lisp dispatcher.

Usage:
    python rust_actr7/fixtures/gui/run_lisp_gui.py
"""

import os
import socket
import subprocess
import sys
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


def main():
    address = home_value("act-r-address.txt", "127.0.0.1")
    if port_open(address, PORT) or port_open("127.0.0.1", PORT):
        raise SystemExit(f"port {PORT} is already in use")

    saved_net = CURRENT_NET.read_bytes() if CURRENT_NET.exists() else None
    ready = Path(tempfile.gettempdir()) / f"actr-gui-ready-{uuid.uuid4().hex}.txt"
    server_out = tempfile.TemporaryFile()
    server_err = tempfile.TemporaryFile()
    server = None
    code = 1
    try:
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
                raise SystemExit("SBCL server exited during startup")
            target = home_value("act-r-address.txt", "127.0.0.1")
            if ready.exists() and port_open(target, PORT):
                break
            time.sleep(0.1)
        else:
            raise SystemExit("SBCL server did not become ready")

        result = subprocess.run(
            [
                "cargo",
                "test",
                "--features",
                "env",
                "--lib",
                "env::app::gui_tests::lisp_",
                "--",
                "--ignored",
                "--nocapture",
                "--test-threads=1",
            ],
            cwd=CRATE,
        )
        code = result.returncode
    finally:
        stop(server)
        time.sleep(0.2)
        server_out.seek(0)
        server_err.seek(0)
        stdout = server_out.read().decode("utf-8", "replace")
        stderr = server_err.read().decode("utf-8", "replace")
        server_out.close()
        server_err.close()
        print(f"\n[server stdout tail]\n{stdout[-4000:].rstrip()}")
        print(f"\n[server stderr tail]\n{stderr[-4000:].rstrip()}")
        if "Invalid detail level" in stdout or "Invalid detail level" in stderr:
            print("\nFAIL: Lisp reported an invalid detail level")
            code = 1
        try:
            ready.unlink()
        except OSError:
            pass
        if saved_net is not None:
            CURRENT_NET.write_bytes(saved_net)

    return code


if __name__ == "__main__":
    raise SystemExit(main())
