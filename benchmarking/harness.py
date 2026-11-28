#!/usr/bin/env python3
"""Server and subprocess plumbing for the benchmark runner.

The RPC benchmarks need a real ACT-R dispatcher.  Both servers write the same
``~/act-r-address.txt`` / ``~/act-r-port-num.txt`` files, so the stock
``actr.py`` client can attach to either one without changes.  This module
starts the chosen server, waits for it, runs client subprocesses, and restores
the user's ACT-R files afterwards.
"""
from __future__ import annotations

import json
import platform
import socket
import subprocess
import sys
import tempfile
import time
import uuid
from pathlib import Path
from typing import Any

HERE = Path(__file__).resolve().parent
ROOT = HERE.parent
ACTR = ROOT / "actr7.x"
CRATE = ROOT / "rust_actr7"
CURRENT_NET = ACTR / "environment" / "gui" / "init" / "05-current-net.tcl"
# ACT-R rewrites these two mode files every time it loads.
LOAD_MODE = ACTR / "support" / "load-mode.lisp"
REQUIRE_MODE = ACTR / "support" / "require-mode.lisp"
DEFAULT_PORT = 2650
CLIENT = HERE / "client.py"


def rust_binary() -> Path:
    """Path to the release ``actr`` binary for the current platform."""
    name = "actr.exe" if sys.platform.startswith("win") else "actr"
    return CRATE / "target" / "release" / name


RUST = rust_binary()


def home_file(name: str) -> Path:
    return Path.home() / name


def read_text(path: Path) -> str | None:
    try:
        return path.read_text(encoding="utf-8")
    except OSError:
        return None


def read_bytes(path: Path) -> bytes | None:
    try:
        return path.read_bytes()
    except OSError:
        return None


def local_hosts() -> list[str]:
    hosts = {"127.0.0.1", "localhost"}
    address = read_text(home_file("act-r-address.txt"))
    if address:
        hosts.add(address.strip())
    try:
        for info in socket.getaddrinfo(socket.gethostname(), None, socket.AF_INET):
            hosts.add(info[4][0])
    except OSError:
        pass
    return [host for host in hosts if host]


def port_open(host: str, port: int, timeout: float = 0.2) -> bool:
    try:
        with socket.create_connection((host, port), timeout=timeout):
            return True
    except OSError:
        return False


def any_port_open(port: int = DEFAULT_PORT) -> list[str]:
    return [host for host in local_hosts() if port_open(host, port)]


def stop(proc: subprocess.Popen[Any]) -> None:
    if proc.poll() is not None:
        return
    proc.terminate()
    try:
        proc.wait(timeout=5)
    except subprocess.TimeoutExpired:
        proc.kill()
        proc.wait(timeout=5)


def read_stream(stream: Any) -> str:
    stream.flush()
    stream.seek(0)
    return stream.read().decode("utf-8", "replace")


def save_actr_files() -> dict[Path, bytes | None]:
    """Remember the files ACT-R rewrites so they can be restored afterwards."""
    saved: dict[Path, bytes | None] = {}
    for path in (
        home_file("act-r-address.txt"),
        home_file("act-r-port-num.txt"),
        CURRENT_NET,
        LOAD_MODE,
        REQUIRE_MODE,
    ):
        saved[path] = read_bytes(path)
    return saved


def restore_actr_files(saved: dict[Path, bytes | None]) -> None:
    for path, data in saved.items():
        try:
            if data is None:
                path.unlink()
            else:
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_bytes(data)
        except OSError:
            pass


class Server:
    """A Lisp or Rust ACT-R RPC dispatcher on the default port."""

    def __init__(self, name: str, timeout: float = 180.0, cpu: int | None = None) -> None:
        if name not in ("lisp", "rust"):
            raise ValueError(f"unknown server {name!r}")
        self.name = name
        self.timeout = timeout
        self.cpu = cpu
        self.proc: subprocess.Popen[Any] | None = None
        self.saved: dict[Path, bytes | None] = {}
        self.ready = Path(tempfile.gettempdir()) / f"actr-bench-ready-{uuid.uuid4().hex}.txt"
        self.stdout: Any = None
        self.stderr: Any = None

    def _save_files(self) -> None:
        self.saved = save_actr_files()
        # Stale connection files from a dead server would otherwise be
        # "restored" after every run.  Keep them only when something is still
        # listening on the saved address/port.
        address = read_text(home_file("act-r-address.txt"))
        port_text = read_text(home_file("act-r-port-num.txt"))
        running = False
        if address and port_text:
            try:
                running = port_open(address.strip(), int(port_text.strip()))
            except ValueError:
                running = False
        if not running:
            self.saved[home_file("act-r-address.txt")] = None
            self.saved[home_file("act-r-port-num.txt")] = None

    def _restore_files(self) -> None:
        restore_actr_files(self.saved)
        try:
            self.ready.unlink()
        except OSError:
            pass

    def _wait_ready(self) -> None:
        deadline = time.time() + self.timeout
        while time.time() < deadline:
            if self.proc is not None and self.proc.poll() is not None:
                out = read_stream(self.stdout) if self.stdout is not None else ""
                err = read_stream(self.stderr) if self.stderr is not None else ""
                raise RuntimeError(
                    f"{self.name} server exited during startup\nstdout:\n{out}\nstderr:\n{err}"
                )
            # ACT-R opens the dispatcher socket before it finishes loading
            # user-loads/*.lisp.  Wait for the marker written after
            # load-act-r.lisp returns, otherwise a client can connect while a
            # user model is half-defined and the system is still "running".
            if self.name == "lisp" and not self.ready.exists():
                time.sleep(0.1)
                continue
            address = read_text(home_file("act-r-address.txt"))
            port_text = read_text(home_file("act-r-port-num.txt"))
            if address and port_text:
                try:
                    port = int(port_text.strip())
                except ValueError:
                    port = DEFAULT_PORT
                if port_open(address.strip(), port):
                    return
            time.sleep(0.1)
        raise RuntimeError(f"{self.name} server did not become ready")

    def start(self) -> "Server":
        busy = any_port_open()
        if busy:
            raise RuntimeError(
                f"port {DEFAULT_PORT} is already in use on {', '.join(busy)}; "
                "stop the running ACT-R server or pass --connect"
            )

        self._save_files()
        for path in (home_file("act-r-address.txt"), home_file("act-r-port-num.txt")):
            try:
                path.unlink()
            except OSError:
                pass

        self.stdout = tempfile.TemporaryFile()
        self.stderr = tempfile.TemporaryFile()

        if self.name == "lisp":
            # Loading ACT-R starts the dispatcher.  The LOOP keeps the process
            # alive; the `ready` file is written only after the load returns.
            ready = self.ready.as_posix()
            command = [
                "sbcl",
                "--load",
                "load-act-r.lisp",
                "--eval",
                (
                    f'(progn (with-open-file (f "{ready}" :direction :output '
                    ":if-exists :supersede :if-does-not-exist :create) "
                    f'(format f "ready") (finish-output f)) '
                    "(loop (sleep 60)))"
                ),
            ]
            cwd = ACTR
            stdin = subprocess.PIPE
        else:
            if not RUST.is_file():
                raise RuntimeError(
                    f"missing release binary {RUST}; run `cargo build --release` "
                    f"in {CRATE}"
                )
            command = [str(RUST), "server"]
            cwd = CRATE
            stdin = subprocess.DEVNULL

        if self.cpu is not None and sys.platform.startswith("linux"):
            command = ["taskset", "-c", str(self.cpu), *command]

        try:
            self.proc = subprocess.Popen(
                command,
                cwd=cwd,
                stdin=stdin,
                stdout=self.stdout,
                stderr=self.stderr,
            )
        except OSError as exc:
            self.close()
            raise RuntimeError(f"could not start {self.name} server: {exc}") from exc

        try:
            self._wait_ready()
        except Exception:
            self.close()
            raise
        return self

    def close(self) -> None:
        if self.proc is not None:
            stop(self.proc)
            self.proc = None
        if self.stdout is not None:
            self.stdout.close()
            self.stdout = None
        if self.stderr is not None:
            self.stderr.close()
            self.stderr = None
        self._restore_files()

    def __enter__(self) -> "Server":
        return self.start()

    def __exit__(self, exc_type: Any, exc: Any, tb: Any) -> None:
        self.close()


def run_client(
    args: list[str],
    *,
    timeout: float | None = None,
    cpu: int | None = None,
) -> tuple[list[dict[str, Any]], dict[str, Any], str]:
    """Run ``client.py`` and return its BENCH_JSON/BENCH_META records."""
    cmd = [sys.executable, str(CLIENT), *args]
    if cpu is not None and sys.platform.startswith("linux"):
        cmd = ["taskset", "-c", str(cpu), *cmd]
    proc = subprocess.run(
        cmd,
        cwd=ROOT,
        capture_output=True,
        text=True,
        timeout=timeout,
    )
    records: list[dict[str, Any]] = []
    meta: dict[str, Any] = {}
    for line in proc.stdout.splitlines():
        if line.startswith("BENCH_JSON "):
            records.append(json.loads(line[len("BENCH_JSON ") :]))
        elif line.startswith("BENCH_META "):
            meta = json.loads(line[len("BENCH_META ") :])
    if proc.returncode != 0 or not records:
        raise RuntimeError(
            "benchmark client failed\n"
            f"command: {' '.join(cmd)}\n"
            f"exit: {proc.returncode}\n"
            f"stdout:\n{proc.stdout}\n"
            f"stderr:\n{proc.stderr}"
        )
    return records, meta, proc.stdout + proc.stderr


def _cpu_model() -> str | None:
    try:
        for line in Path("/proc/cpuinfo").read_text(encoding="utf-8").splitlines():
            if line.lower().startswith("model name"):
                return line.split(":", 1)[1].strip()
    except OSError:
        pass
    return None


def _command_output(cmd: list[str], cwd: Path | None = None) -> str | None:
    try:
        return subprocess.run(
            cmd,
            cwd=cwd,
            capture_output=True,
            text=True,
            timeout=10,
        ).stdout.strip().splitlines()[0]
    except (OSError, subprocess.SubprocessError, IndexError):
        return None


def metadata() -> dict[str, Any]:
    data: dict[str, Any] = {
        "platform": platform.platform(),
        "machine": platform.machine(),
        "processor": platform.processor(),
        "cpu_model": _cpu_model(),
        "python": sys.version.split()[0],
        "python_implementation": platform.python_implementation(),
        "rust_binary": str(RUST) if RUST.is_file() else None,
        "git_commit": _command_output(["git", "rev-parse", "--short", "HEAD"], ROOT),
        "sbcl": _command_output(["sbcl", "--version"]),
        "rustc": _command_output(["rustc", "--version"]),
        "cargo": _command_output(["cargo", "--version"]),
    }
    try:
        import rust_actr

        data["rust_actr"] = rust_actr.__version__
    except Exception:
        data["rust_actr"] = None
    return data
