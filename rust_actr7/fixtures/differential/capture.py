#!/usr/bin/env python3
import argparse
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
RUST = ROOT / "rust_actr7" / "target" / "debug" / "actr.exe"
CURRENT_NET = ACTR / "environment" / "gui" / "init" / "05-current-net.tcl"
DEFAULT_PORT = 2650
SBCL_READY = "HARNESS-READY"


def read_home_file(name, default=None):
    try:
        return (Path.home() / name).read_text().strip()
    except OSError:
        return default


def local_hosts():
    hosts = {"127.0.0.1", "localhost"}
    address = read_home_file("act-r-address.txt")
    if address:
        hosts.add(address)
    try:
        for info in socket.getaddrinfo(socket.gethostname(), None, socket.AF_INET):
            hosts.add(info[4][0])
    except OSError:
        pass
    return [host for host in hosts if host]


def connection_target():
    address = read_home_file("act-r-address.txt")
    if not address:
        return None
    try:
        port = int(read_home_file("act-r-port-num.txt", str(DEFAULT_PORT)))
    except ValueError:
        return None
    return address, port


def port_open(host, port):
    try:
        with socket.create_connection((host, port), timeout=0.2):
            return True
    except OSError:
        return False


def any_port_open():
    return any(port_open(host, DEFAULT_PORT) for host in local_hosts())


def read_stream(stream):
    stream.flush()
    stream.seek(0)
    return stream.read().decode("utf-8", "replace")


def wait_for_client_ready(proc, timeout, marker=None):
    deadline = time.time() + timeout
    while time.time() < deadline:
        if proc.poll() is not None:
            return False
        if marker is not None and not marker.exists():
            time.sleep(0.1)
            continue
        target = connection_target()
        if target is not None and port_open(*target):
            return True
        time.sleep(0.1)
    return False


def stop(proc):
    if proc is None or proc.poll() is not None:
        return
    if os.name == "nt":
        subprocess.run(
            ["taskkill", "/PID", str(proc.pid), "/T", "/F"],
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
        )
        try:
            proc.wait(timeout=5)
        except subprocess.TimeoutExpired:
            pass
        return
    proc.terminate()
    try:
        proc.wait(timeout=5)
    except subprocess.TimeoutExpired:
        proc.kill()
        proc.wait(timeout=5)


class Server:
    def __init__(self, name, timeout=180.0):
        self.name = name
        self.timeout = timeout
        self.saved_net = None
        self.server_out = None
        self.server_err = None
        self.ready_path = None
        self.proc = None
        self.stdout = ""
        self.stderr = ""

    def __enter__(self):
        self.start()
        return self

    def __exit__(self, exc_type, exc, tb):
        self.close()

    def start(self):
        if any_port_open():
            raise RuntimeError(f"port {DEFAULT_PORT} is already in use")

        self.saved_net = CURRENT_NET.read_bytes() if CURRENT_NET.exists() else None
        self.server_out = tempfile.TemporaryFile()
        self.server_err = tempfile.TemporaryFile()
        self.ready_path = Path(tempfile.gettempdir()) / f"actr-ready-{uuid.uuid4().hex}.txt"

        if self.name == "sbcl":
            ready = self.ready_path.as_posix()
            self.proc = subprocess.Popen(
                [
                    "sbcl",
                    "--load",
                    "load-act-r.lisp",
                    "--eval",
                    f'(progn (with-open-file (f "{ready}" :direction :output '
                    ':if-exists :supersede :if-does-not-exist :create) '
                    f'(format f "{SBCL_READY}") (finish-output f)) '
                    "(loop (sleep 60)))",
                ],
                cwd=ACTR,
                stdin=subprocess.PIPE,
                stdout=self.server_out,
                stderr=self.server_err,
            )
        else:
            if not RUST.is_file():
                raise RuntimeError(f"missing Rust binary: {RUST}")
            self.proc = subprocess.Popen(
                [str(RUST), "server"],
                cwd=ROOT,
                stdin=subprocess.DEVNULL,
                stdout=self.server_out,
                stderr=self.server_err,
            )

        marker = self.ready_path if self.name == "sbcl" else None
        if not wait_for_client_ready(self.proc, self.timeout, marker):
            self.close()
            raise RuntimeError(f"{self.name} server did not become ready on port {DEFAULT_PORT}")

    def run(self, script, timeout=None):
        return subprocess.run(
            [sys.executable, str(script)],
            cwd=ROOT,
            capture_output=True,
            text=True,
            timeout=timeout or self.timeout,
        )

    def close(self):
        if self.proc is not None:
            stop(self.proc)
            time.sleep(0.2)
        if self.server_out is not None:
            self.stdout = read_stream(self.server_out)
            self.server_out.close()
            self.server_out = None
        if self.server_err is not None:
            self.stderr = read_stream(self.server_err)
            self.server_err.close()
            self.server_err = None
        if self.ready_path is not None:
            try:
                self.ready_path.unlink()
            except OSError:
                pass
            self.ready_path = None
        if self.saved_net is not None:
            CURRENT_NET.write_bytes(self.saved_net)
            self.saved_net = None


def run(server, timeout, test):
    with Server(server, timeout) as srv:
        client = srv.run(test, timeout)
    return {
        "server": server,
        "server_stdout": srv.stdout,
        "server_stderr": srv.stderr,
        "client": client,
    }


def tail(text, lines=30):
    rows = text.rstrip().splitlines()
    if len(rows) <= lines:
        return text.rstrip()
    return "\n".join(rows[-lines:])


def show(result, full_server_output):
    print(f"\n===== {result['server']} server stdout =====")
    text = result["server_stdout"] if full_server_output else tail(result["server_stdout"])
    print(text)
    if result["server_stderr"].strip():
        print(f"\n===== {result['server']} server stderr =====")
        print(result["server_stderr"].rstrip())
    client = result["client"]
    print(f"\n===== {result['server']} client stdout =====")
    print(client.stdout.rstrip())
    if client.stderr.strip():
        print(f"\n===== {result['server']} client stderr =====")
        print(client.stderr.rstrip())


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--server", choices=("sbcl", "rust", "both"), default="both")
    parser.add_argument("--timeout", type=float, default=180.0)
    parser.add_argument("--test", type=Path, required=True)
    parser.add_argument("--full-server-output", action="store_true")
    args = parser.parse_args()

    servers = ("sbcl", "rust") if args.server == "both" else (args.server,)
    failed = False
    for server in servers:
        result = run(server, args.timeout, args.test)
        show(result, args.full_server_output)
        if result["client"].returncode != 0:
            failed = True
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main())
