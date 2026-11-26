"""Shared harness for the local Tk Environment compatibility suite.

Each scenario runs in its own process.  The parent runner builds the server,
copies the Tk Environment into ``.tmp`` (never editing ``actr7.x``), and passes
the paths to the child.  The child starts a private server, loads the *real*
Tcl client code from the copy, and runs one scenario.

The harness deliberately does not source ``server.tcl`` or the GUI init files:
those read/write user settings, open the full control panel, and auto-connect.
Instead it loads ``new-server.tcl`` (the real RPC transport) plus ``shims.tcl``
and lets each scenario source only the dialog files it exercises.
"""

from __future__ import annotations

import contextlib
import os
import shutil
import socket
import subprocess
import sys
import time
import tkinter
import traceback
from dataclasses import dataclass, field
from pathlib import Path
from typing import Callable

HERE = Path(__file__).resolve().parent
FIXTURES = HERE.parent
RUST_DIR = FIXTURES.parent
REPO_ROOT = RUST_DIR.parent
ACTR7 = REPO_ROOT / "actr7.x"
GUI_SRC = ACTR7 / "environment" / "gui"
TMP_ROOT = REPO_ROOT / ".tmp" / "tk-env"
SHIMS = HERE / "shims.tcl"
DEFAULT_MODEL = FIXTURES / "models" / "tutorial" / "unit1" / "addition.lisp"


def free_port() -> int:
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as s:
        s.bind(("127.0.0.1", 0))
        return s.getsockname()[1]


def prepare_binary(binary: str | None, target_dir: Path) -> Path:
    if binary:
        path = Path(binary).resolve()
        if not path.is_file():
            raise SystemExit(f"server binary not found: {path}")
        return path

    exe = "actr.exe" if os.name == "nt" else "actr"
    target_dir.mkdir(parents=True, exist_ok=True)
    print(f"building actr server into {target_dir} ...", flush=True)
    cmd = [
        "cargo",
        "build",
        "--bin",
        "actr",
        "--manifest-path",
        str(RUST_DIR / "Cargo.toml"),
        "--target-dir",
        str(target_dir),
    ]
    if subprocess.run(cmd, cwd=RUST_DIR).returncode != 0:
        raise SystemExit("cargo build failed")
    path = target_dir / "debug" / exe
    if not path.is_file():
        raise SystemExit(f"built server binary not found: {path}")
    return path


def prepare_gui(run_dir: Path) -> Path:
    if not GUI_SRC.is_dir():
        raise SystemExit(f"Tk Environment sources not found: {GUI_SRC}")
    dst = run_dir / "gui"
    if dst.exists():
        shutil.rmtree(dst)
    shutil.copytree(GUI_SRC, dst)
    return dst


def wait_for_port(port: int, proc: subprocess.Popen, timeout: float = 30.0) -> None:
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        if proc.poll() is not None:
            raise RuntimeError(f"server exited early with code {proc.returncode}")
        try:
            with socket.create_connection(("127.0.0.1", port), timeout=0.5):
                return
        except OSError:
            time.sleep(0.1)
    raise RuntimeError(f"server did not listen on 127.0.0.1:{port} within {timeout}s")


@dataclass
class Server:
    proc: subprocess.Popen
    port: int
    log_path: Path
    log: object = field(repr=False)

    def stop(self) -> None:
        if self.proc.poll() is None:
            self.proc.terminate()
            try:
                self.proc.wait(timeout=5)
            except subprocess.TimeoutExpired:
                self.proc.kill()
        with contextlib.suppress(Exception):
            self.log.close()


def start_server(binary: Path, run_dir: Path, seed: tuple[int, int] = (100, 0)) -> Server:
    port = free_port()
    log_path = run_dir / "server.log"
    log = open(log_path, "w", encoding="utf-8")
    env = os.environ.copy()
    env["RPC_DEBUG"] = "1"
    cmd = [
        str(binary),
        "server",
        "--host",
        "127.0.0.1",
        "--port",
        str(port),
        "--no-port-file",
        "--seed",
        str(seed[0]),
        str(seed[1]),
    ]
    proc = subprocess.Popen(
        cmd,
        cwd=RUST_DIR,
        stdout=subprocess.DEVNULL,
        stderr=log,
        env=env,
    )
    wait_for_port(port, proc)
    return Server(proc=proc, port=port, log_path=log_path, log=log)


@dataclass(frozen=True)
class Scenario:
    name: str
    run: Callable[["Context"], None]
    tags: tuple[str, ...] = ()
    timeout: float = 90.0


class Context:
    """One live server + one hidden Tk interpreter running the Tcl client."""

    def __init__(
        self,
        *,
        gui_dir: Path,
        run_dir: Path,
        server: Server | None,
        addr: str | None,
        model: Path,
        run_time: str,
        detail: str,
        dump_rpc: bool,
    ) -> None:
        self.gui_dir = gui_dir
        self.run_dir = run_dir
        self.server = server
        self.addr = addr
        self.model = model
        self.run_time = run_time
        self.detail = detail
        self.dump_rpc = dump_rpc
        self.root: tkinter.Tk | None = None
        self.tcl = None
        self._old_cwd: Path | None = None

    # -- lifecycle -----------------------------------------------------
    def connect(self) -> None:
        host = "127.0.0.1"
        if self.addr:
            h, _, p = self.addr.rpartition(":")
            host, port = (h or "127.0.0.1"), int(p)
        elif self.server is not None:
            port = self.server.port
        else:
            raise RuntimeError("no server address")

        self._old_cwd = Path.cwd()
        os.chdir(self.gui_dir)
        self.root = tkinter.Tk()
        self.root.withdraw()
        self.tcl = self.root.tk
        self.tcl.eval('source "new-server.tcl"')
        self.tcl.call("source", str(SHIMS))
        self.tcl.call("set", "options_array(use_localhost)", "0")
        self.tcl.call("set", "up_and_running", "0")
        print(f"connecting Tcl Environment client to {host}:{port}", flush=True)
        self.tcl.call("start_connection", host, str(port))

    def close(self) -> None:
        if self.tcl is not None:
            with contextlib.suppress(Exception):
                self.tcl.eval("catch {close $environment_socket}")
        if self.root is not None:
            with contextlib.suppress(Exception):
                self.root.destroy()
        if self._old_cwd is not None:
            with contextlib.suppress(Exception):
                os.chdir(self._old_cwd)
        if self.server is not None:
            self.server.stop()

    # -- Tcl helpers ---------------------------------------------------
    def eval(self, script: str) -> str:
        assert self.tcl is not None
        return self.tcl.eval(script)

    def _tuple(self, value) -> tuple:
        """Normalize a Tcl result.  ``_tkinter`` already converts many Tcl
        lists to Python tuples, so ``splitlist`` must only be used on strings."""
        if isinstance(value, (tuple, list)):
            return tuple(value)
        return tuple(self.tcl.splitlist(str(value)))

    def values(self, value) -> tuple[str, ...]:
        """Flatten one Tcl list level into strings."""
        return tuple(str(v) for v in self._tuple(value))

    def call(self, proc: str, *args) -> tuple:
        assert self.tcl is not None
        return self._tuple(self.tcl.call(proc, *args))

    def call_act_r_command(self, name: str, model: str = "nil", *params) -> tuple:
        assert self.tcl is not None
        plist = self.tcl.call("list", *[str(p) for p in params])
        return self._tuple(self.tcl.call("call_act_r_command", name, model, plist))

    def call_act_r_command_raw(self, name: str, model: str = "nil", *params) -> str:
        assert self.tcl is not None
        plist = self.tcl.call("list", *[str(p) for p in params])
        return str(self.tcl.call("call_act_r_command", name, model, plist))

    def call_with_errors(self, name: str, model: str = "nil", *params) -> tuple:
        assert self.tcl is not None
        plist = self.tcl.call("list", *[str(p) for p in params])
        value = self.tcl.call(
            "call_act_r_command_with_error_messages", name, model, plist
        )
        return self._tuple(value)

    def send(self, method: str, *params) -> tuple:
        assert self.tcl is not None
        plist = self.tcl.call("list", *[str(p) for p in params])
        return self._tuple(self.tcl.call("send_cmd", method, plist))

    def tcl_list(self, *items) -> str:
        assert self.tcl is not None
        return self.tcl.call("list", *[str(i) for i in items])

    def setvar(self, name: str, value: str) -> None:
        assert self.tcl is not None
        self.tcl.setvar(name, value)

    def getvar(self, name: str) -> str:
        assert self.tcl is not None
        return self.tcl.getvar(name)

    def set_global(self, name: str, value: str) -> None:
        assert self.tcl is not None
        self.tcl.call("set", name, value)

    def get_global(self, name: str) -> str:
        assert self.tcl is not None
        return self.tcl.call("set", name)

    def source_dialog(self, filename: str) -> None:
        self.tcl.call("source", str(self.gui_dir / "dialogs" / filename))

    def source_gui(self, filename: str) -> None:
        self.tcl.call("source", str(self.gui_dir / filename))

    # -- widget helpers ------------------------------------------------
    def update(self) -> None:
        assert self.root is not None
        self.root.update_idletasks()
        self.root.update()

    def wait_until(self, predicate: Callable[[], bool], timeout: float = 10.0) -> None:
        deadline = time.monotonic() + timeout
        while time.monotonic() < deadline:
            self.update()
            if predicate():
                return
            time.sleep(0.02)
        raise TimeoutError("condition not reached before timeout")

    def text(self, widget: str) -> str:
        assert self.tcl is not None
        return self.tcl.call(widget, "get", "1.0", "end-1c")

    def list_items(self, widget: str) -> tuple[str, ...]:
        assert self.tcl is not None
        return self.values(self.tcl.call(widget, "get", "0", "end"))

    def entry(self, widget: str) -> str:
        assert self.tcl is not None
        return self.tcl.call(widget, "get")

    def invoke(self, widget: str) -> None:
        assert self.tcl is not None
        self.tcl.call(widget, "invoke")

    def find_window(self, prefix: str) -> str:
        assert self.tcl is not None
        children = self.values(self.tcl.call("winfo", "children", "."))
        for child in children:
            if child.startswith(prefix):
                return child
        raise AssertionError(f"no toplevel starting with {prefix!r}: {children}")

    def run_updates(self) -> None:
        with contextlib.suppress(Exception):
            self.eval("run_shim_updates")

    def message_boxes(self) -> tuple[str, ...]:
        if self.eval("info exists ::tk_message_boxes") != "1":
            return ()
        return tuple(
            line for line in self.eval("set ::tk_message_boxes").splitlines() if line
        )

    def clear_message_boxes(self) -> None:
        self.set_global("::tk_message_boxes", "")

    # -- convenience ---------------------------------------------------
    def load_model(self, path: Path | None = None) -> tuple[str, ...]:
        model = path or self.model
        return self.call_with_errors("load-act-r-code", "nil", model.as_posix(), "0")

    def run_model(self, seconds: str | None = None) -> tuple[str, ...]:
        return self.call_with_errors("run", "nil", seconds or self.run_time)

    def current_model(self) -> str:
        got = self.call_act_r_command("current-model")
        return got[0] if got else ""


def run_internal(args) -> int:
    from scenarios import all_scenarios

    scenario = next((s for s in all_scenarios() if s.name == args.scenario), None)
    if scenario is None:
        print(f"unknown scenario: {args.scenario}", file=sys.stderr)
        return 2

    run_dir = Path(args.run_dir).resolve()
    run_dir.mkdir(parents=True, exist_ok=True)
    gui_dir = Path(args.gui_dir).resolve()
    model = Path(args.model).resolve()

    server = None
    ctx = None
    try:
        if not args.addr:
            binary = Path(args.binary).resolve()
            server = start_server(binary, run_dir)
            print(f"server log: {server.log_path}", flush=True)
        ctx = Context(
            gui_dir=gui_dir,
            run_dir=run_dir,
            server=server,
            addr=args.addr,
            model=model,
            run_time=args.run_time,
            detail=args.detail,
            dump_rpc=args.dump_rpc,
        )
        ctx.connect()
        scenario.run(ctx)
        print(f"PASS {scenario.name}", flush=True)
        return 0
    except Exception:
        print(f"FAIL {scenario.name}", flush=True)
        traceback.print_exc()
        if ctx is not None and ctx.server is not None:
            with contextlib.suppress(Exception):
                tail = ctx.server.log_path.read_text(encoding="utf-8", errors="replace")
                print("--- server log tail ---")
                print("\n".join(tail.splitlines()[-80:]))
        return 1
    finally:
        if ctx is not None:
            ctx.close()
        elif server is not None:
            server.stop()
