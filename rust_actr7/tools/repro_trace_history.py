#!/usr/bin/env python3
"""Reproduce the Tk Environment's trace-history fetch against a Rust server.

The tool starts its own `actr server` on a free localhost port (no port files),
copies the Tk Environment's `gui` directory into `.tmp/` (so the vendored
`actr7.x` tree is never modified), and drives the *real* Tcl client code from
that copy: `new-server.tcl` (RPC transport + list2json + call_act_r_command)
and `json.tcl`.  It then follows the Environment's actual history sequence:

    record-history -> load-act-r-code -> run -> history-data-available
    -> start-incremental-history-data -> get-incremental-history-data loop

Each chunk is printed and parsed with the Environment's own JSON parser, so a
failure in the Tcl client or in the server's wire format is reported directly.

Usage:
    python tools/repro_trace_history.py
    python tools/repro_trace_history.py --detail all --run-time 10.0
    python tools/repro_trace_history.py --addr 127.0.0.1:2650
    python tools/repro_trace_history.py --binary target/debug/actr.exe
"""

from __future__ import annotations

import argparse
import os
import shutil
import socket
import subprocess
import sys
import time
import tkinter
from pathlib import Path

HERE = Path(__file__).resolve().parent
RUST_DIR = HERE.parent
REPO_ROOT = RUST_DIR.parent
ACTR7 = REPO_ROOT / "actr7.x"
TMP = REPO_ROOT / ".tmp" / "repro-trace-history"
DEFAULT_MODEL = ACTR7 / "tutorial" / "unit1" / "addition.lisp"


def free_port() -> int:
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as s:
        s.bind(("127.0.0.1", 0))
        return s.getsockname()[1]


def wait_for_port(port: int, proc: subprocess.Popen, timeout: float = 30.0) -> None:
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        if proc.poll() is not None:
            raise SystemExit(
                f"server exited early with code {proc.returncode}; see {TMP / 'server.log'}"
            )
        try:
            with socket.create_connection(("127.0.0.1", port), timeout=0.5):
                return
        except OSError:
            time.sleep(0.1)
    raise SystemExit(f"server did not listen on 127.0.0.1:{port} within {timeout}s")


def build_server(args: argparse.Namespace) -> Path:
    if args.binary:
        binary = Path(args.binary).resolve()
        if not binary.is_file():
            raise SystemExit(f"server binary not found: {binary}")
        return binary

    exe = "actr.exe" if os.name == "nt" else "actr"
    target_dir = TMP / "cargo-target"
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
    result = subprocess.run(cmd, cwd=RUST_DIR)
    if result.returncode != 0:
        raise SystemExit("cargo build failed")
    binary = target_dir / "debug" / exe
    if not binary.is_file():
        raise SystemExit(f"built server binary not found: {binary}")
    return binary


def copy_tk_env(keep: bool) -> Path:
    src = ACTR7 / "environment" / "gui"
    if not src.is_dir():
        raise SystemExit(f"Tk Environment sources not found: {src}")
    dst = TMP / "tk-env" / "gui"
    if dst.exists() and not keep:
        shutil.rmtree(dst.parent)
    dst.parent.mkdir(parents=True, exist_ok=True)
    shutil.copytree(src, dst, dirs_exist_ok=True)
    return dst


def tcl_catch(tcl, script: str, label: str) -> None:
    """Evaluate a Tcl snippet, printing errors instead of aborting the run."""
    wrapped = (
        "if {[catch {\n" + script + "\n} __err __opts]} {\n"
        '  puts "' + label + ': ERROR <$__err>"\n'
        '  puts "' + label + ': TRACE"\n'
        '  puts [dict get $__opts -errorinfo]\n'
        "}\n"
    )
    tcl.eval(wrapped)


def drive(args: argparse.Namespace, port: int, gui_dir: Path, model: Path) -> None:
    # new-server.tcl sources json.tcl relative to the process cwd.
    os.chdir(gui_dir)

    root = tkinter.Tk()
    root.withdraw()
    tcl = root.tk
    tcl.eval('source "new-server.tcl"')
    tcl.eval("set options_array(use_localhost) 0")
    tcl.eval("set up_and_running 0")

    print(f"connecting Tcl Environment client to 127.0.0.1:{port}")
    tcl.eval(f"start_connection 127.0.0.1 {port}")

    # Tcl's list2json does not escape backslashes, and the Windows file
    # dialogs hand Tk forward-slash paths.  Match that wire shape.
    tcl.setvar("repro_model", model.as_posix())
    tcl.setvar("repro_run_time", args.run_time)
    tcl.setvar("repro_detail", args.detail)
    tcl.setvar("repro_use_smart_load", "0")

    print("\n--- record-history ---")
    tcl_catch(
        tcl,
        'puts "record-history result=<[call_act_r_command '
        '{record-history} nil [list "trace-history"]]>"',
        "record-history",
    )

    print("\n--- load-act-r-code ---")
    tcl_catch(
        tcl,
        "puts \"load result=<[call_act_r_command_with_error_messages "
        '{load-act-r-code} nil [list $repro_model $repro_use_smart_load]]>"',
        "load",
    )

    print("\n--- run ---")
    tcl_catch(
        tcl,
        "puts \"run result=<[call_act_r_command_with_error_messages "
        '{run} "nil" [list $repro_run_time]]>"',
        "run",
    )

    print("\n--- history-data-available ---")
    tcl_catch(
        tcl,
        "puts \"available=<[call_act_r_command "
        '{history-data-available} nil '
        '[list "trace-history" nil nil $repro_detail]]>"',
        "available",
    )

    print("\n--- start-incremental-history-data ---")
    tcl_catch(
        tcl,
        "set repro_id [call_act_r_command "
        '{start-incremental-history-data} nil '
        '[list "trace-history" 16000 false nil nil $repro_detail]]\n'
        'puts "job id=<$repro_id>"',
        "start",
    )

    print("\n--- get-incremental-history-data loop (Environment semantics) ---")
    tcl_catch(
        tcl,
        # Mirrors get_incremental_history from
        # dialogs/60-ctrl-panel-recorded-label.tcl, with instrumentation.
        "if {![info exists repro_id] || $repro_id == {}} {\n"
        '  puts "no job id; skipping fetch"\n'
        "} else {\n"
        "  set start [call_act_r_command "
        '{get-incremental-history-data} nil [list $repro_id]]\n'
        "  set n 0\n"
        '  while {$start != "" && $n < 1000} {\n'
        "    incr n\n"
        "    set flat [lindex $start 0]\n"
        '    puts "chunk $n bytes=[string length $flat] head=<[string range $flat 0 160]>"\n'
        "    if {[catch {json::json2dict $flat} parsed]} {\n"
        '      puts "chunk $n JSON ERROR: <$parsed>"\n'
        '      puts "chunk $n flat=<[string range $flat 0 1000]>"\n'
        "      break\n"
        "    }\n"
        '    puts "chunk $n records=[llength $parsed]"\n'
        "    set start [call_act_r_command "
        '{get-incremental-history-data} nil [list $repro_id]]\n'
        "  }\n"
        '  puts "loop done chunks=$n terminated=[expr {$start == ""}] '
        'tail=<[string range $start 0 80]>"\n'
        "}\n",
        "fetch",
    )

    if args.save:
        target = (TMP / args.save).resolve()
        target.parent.mkdir(parents=True, exist_ok=True)
        tcl.setvar("repro_save", target.as_posix())
        print(f"\n--- save-history-data -> {target} ---")
        tcl_catch(
            tcl,
            "puts \"save result=<[call_act_r_command_with_error_messages "
            '{save-history-data} nil '
            '[list "trace-history" $repro_save "repro" nil nil $repro_detail]]>"',
            "save",
        )
        print(f"saved={target.is_file()} size={target.stat().st_size if target.is_file() else 0}")

    tcl.eval('catch {close $environment_socket}')
    root.destroy()


def main() -> int:
    # Tcl's puts writes directly to the C-level stdout; keep Python prints in
    # order with it.
    sys.stdout.reconfigure(line_buffering=True)
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--model", default=str(DEFAULT_MODEL), help="model file to load")
    parser.add_argument("--run-time", default="10.0", help="run seconds (sent as a Tcl string)")
    parser.add_argument(
        "--detail",
        default="low",
        choices=["low", "medium", "high", "all", "t"],
        help="trace detail passed to the history commands",
    )
    parser.add_argument("--addr", help="connect to an existing server instead of starting one")
    parser.add_argument("--binary", help="use this actr binary instead of building")
    parser.add_argument("--keep-server", action="store_true", help="leave a spawned server running")
    parser.add_argument("--keep-env", action="store_true", help="keep the copied Tk env")
    parser.add_argument("--save", help="also save history to this file under .tmp")
    args = parser.parse_args()

    if args.detail == "all":
        args.detail = "t"

    model = Path(args.model).resolve()
    if not model.is_file():
        raise SystemExit(f"model not found: {model}")

    TMP.mkdir(parents=True, exist_ok=True)
    gui_dir = copy_tk_env(args.keep_env)

    server: subprocess.Popen | None = None
    log = None
    try:
        if args.addr:
            host, _, port_s = args.addr.rpartition(":")
            if not host:
                host = "127.0.0.1"
            port = int(port_s)
            print(f"using existing server at {host}:{port}")
        else:
            port = free_port()
            binary = build_server(args)
            log_path = TMP / "server.log"
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
                "100",
                "0",
            ]
            print(f"starting server: {' '.join(cmd)}")
            server = subprocess.Popen(
                cmd,
                cwd=RUST_DIR,
                stdout=subprocess.DEVNULL,
                stderr=log,
                env=env,
            )
            wait_for_port(port, server)
            print(f"server log: {log_path}")

        drive(args, port, gui_dir, model)
    finally:
        if log is not None:
            log.flush()
        if server is not None and not args.keep_server:
            server.terminate()
            try:
                server.wait(timeout=5)
            except subprocess.TimeoutExpired:
                server.kill()
        if not args.keep_env:
            shutil.rmtree(TMP / "tk-env", ignore_errors=True)
    return 0


if __name__ == "__main__":
    sys.exit(main())
