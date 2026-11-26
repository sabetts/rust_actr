#!/usr/bin/env python3
"""Local Tk Environment compatibility suite for the Rust ACT-R server.

Each scenario starts its own private ``actr server`` and drives the *copied*
Tcl Environment (``new-server.tcl`` plus selected dialog procs).  ``actr7.x``
is never modified; copies and logs live under ``.tmp/tk-env``.

Run from the crate directory:

    python fixtures/tk-env/run.py --list
    python fixtures/tk-env/run.py                       # all scenarios
    python fixtures/tk-env/run.py --scenario trace-history
    python fixtures/tk-env/run.py --filter history
    python fixtures/tk-env/run.py --audit               # Tcl command inventory

Scenarios are exploratory.  When one finds a bug, write a Rust regression test
under ``tests/`` or ``src/rpc/session.rs``; do not rely on the Tk suite as the
long-term gate.
"""

from __future__ import annotations

import argparse
import json
import re
import socket
import subprocess
import sys
import time
from datetime import datetime
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))

import harness  # noqa: E402
from harness import Scenario, run_internal  # noqa: E402
from scenarios import all_scenarios  # noqa: E402

COMMAND_PATTERNS = [
    re.compile(
        r'call_act_r_command(?:_with_error_messages|_string_only)?\s+'
        r'"([A-Za-z0-9?*!<>=+_-]+)"'
    ),
    re.compile(r'call_act_r_command(?:_with_error_messages|_string_only)?\s+'
               r'\{([A-Za-z0-9?*!<>=+_-]+)\}'),
    re.compile(r'send_cmd\s+"([A-Za-z0-9?*!<>=+_-]+)"'),
    re.compile(r'send_cmd\s+\{([A-Za-z0-9?*!<>=+_-]+)\}'),
]

# RPC method names, not ACT-R evaluate commands.
RPC_METHODS = {
    "add",
    "check",
    "close",
    "evaluate",
    "list-commands",
    "list-connections",
    "monitor",
    "remove",
    "remove-monitor",
    "set-name",
}

# Tcl Environment features the Rust dispatcher deliberately does not provide.
KNOWN_UNSUPPORTED = {
    "Python-import-from-file": "Load Python module requires the Lisp Environment",
}


def select(scenarios: list[Scenario], args) -> list[Scenario]:
    chosen = scenarios
    if args.scenario:
        chosen = [s for s in chosen if s.name == args.scenario]
    if args.filter:
        needle = args.filter.lower()
        chosen = [
            s
            for s in chosen
            if needle in s.name.lower() or any(needle in t.lower() for t in s.tags)
        ]
    return chosen


def print_list(scenarios: list[Scenario]) -> None:
    width = max((len(s.name) for s in scenarios), default=0)
    for s in scenarios:
        tags = ", ".join(s.tags)
        print(f"{s.name:<{width}}  {tags}")


def audit(args) -> int:
    """Compare literal command names in the Tcl dialogs with the live server."""
    run_dir = harness.TMP_ROOT / "audit" / datetime.now().strftime("%Y%m%d-%H%M%S")
    run_dir.mkdir(parents=True, exist_ok=True)
    binary = harness.prepare_binary(args.binary, run_dir / "cargo-target")
    server = harness.start_server(binary, run_dir)
    try:
        names = {}
        for path in sorted(harness.GUI_SRC.rglob("*")):
            if path.suffix not in (".tcl", ".tcx"):
                continue
            text = path.read_text(encoding="utf-8", errors="replace")
            for pattern in COMMAND_PATTERNS:
                for name in pattern.findall(text):
                    names.setdefault(name, set()).add(str(path.relative_to(harness.GUI_SRC)))

        command_set = rpc_list_commands(server.port)
        missing = sorted(
            n for n in names if n not in command_set and n not in RPC_METHODS
        )
        print(f"tcl command literals: {len(names)}")
        print(f"server commands:      {len(command_set)}")

        known = [n for n in missing if n in KNOWN_UNSUPPORTED]
        unexpected = [n for n in missing if n not in KNOWN_UNSUPPORTED]
        if known:
            print("\nknown unsupported:")
            for name in known:
                print(f"  {name:<32} {KNOWN_UNSUPPORTED[name]}")
        if unexpected:
            print("\nmissing on server:")
            for name in unexpected:
                where = ", ".join(sorted(names[name])[:3])
                print(f"  {name:<32} {where}")
            return 1
        print("no unexpected missing command names")
        return 0
    finally:
        server.stop()


def rpc_list_commands(port: int) -> set[str]:
    deadline = time.monotonic() + 10
    with socket.create_connection(("127.0.0.1", port), timeout=5) as s:
        s.settimeout(10)
        msg = {
            "method": "list-commands",
            "params": [],
            "id": 1,
        }
        s.sendall((json.dumps(msg) + chr(4)).encode("utf-8"))
        buf = b""
        while time.monotonic() < deadline:
            chunk = s.recv(8192)
            if not chunk:
                break
            buf += chunk
            if chr(4).encode() in buf:
                break
    body = buf.split(chr(4).encode(), 1)[0].decode("utf-8")
    reply = json.loads(body)
    if reply.get("error"):
        raise SystemExit(f"list-commands failed: {reply['error']}")
    rows = reply.get("result") or []
    rows = rows[0] if rows and isinstance(rows[0], list) else rows
    return {row["name"] for row in rows if isinstance(row, dict) and "name" in row}


def run_parent(args) -> int:
    scenarios = all_scenarios()
    if args.list:
        print_list(scenarios)
        return 0

    chosen = select(scenarios, args)
    if not chosen:
        print("no scenarios matched", file=sys.stderr)
        return 2

    stamp = datetime.now().strftime("%Y%m%d-%H%M%S")
    run_dir = harness.TMP_ROOT / "runs" / stamp
    run_dir.mkdir(parents=True, exist_ok=True)
    binary = harness.prepare_binary(args.binary, run_dir / "cargo-target")
    gui_dir = harness.prepare_gui(run_dir)
    print(f"run directory: {run_dir}", flush=True)

    failed = []
    for scenario in chosen:
        scenario_dir = run_dir / scenario.name.replace("/", "_").replace(" ", "_")
        scenario_dir.mkdir(parents=True, exist_ok=True)
        cmd = [
            sys.executable,
            str(Path(__file__).resolve()),
            "--internal",
            "--scenario",
            scenario.name,
            "--binary",
            str(binary),
            "--gui-dir",
            str(gui_dir),
            "--run-dir",
            str(scenario_dir),
            "--model",
            str(Path(args.model).resolve()),
            "--run-time",
            args.run_time,
            "--detail",
            args.detail,
        ]
        if args.addr:
            cmd += ["--addr", args.addr]
        if args.dump_rpc:
            cmd += ["--dump-rpc"]

        print(f"\n=== {scenario.name} ===", flush=True)
        try:
            result = subprocess.run(
                cmd,
                cwd=HERE,
                timeout=scenario.timeout + 30,
                capture_output=True,
                text=True,
            )
            output = (result.stdout or "") + (result.stderr or "")
            print(output, end="" if output.endswith("\n") else "\n")
            if result.returncode != 0:
                failed.append(scenario.name)
        except subprocess.TimeoutExpired as e:
            print(f"TIMEOUT {scenario.name} after {scenario.timeout + 30:.0f}s")
            if e.stdout:
                print(e.stdout if isinstance(e.stdout, str) else e.stdout.decode())
            if e.stderr:
                print(e.stderr if isinstance(e.stderr, str) else e.stderr.decode())
            failed.append(scenario.name)

    print(f"\n{len(chosen) - len(failed)}/{len(chosen)} scenarios passed")
    if failed:
        print("failed: " + ", ".join(failed))
        return 1
    return 0


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--list", action="store_true", help="list scenarios")
    parser.add_argument("--scenario", help="run one scenario by name")
    parser.add_argument("--filter", help="substring match on name or tag")
    parser.add_argument("--model", default=str(harness.DEFAULT_MODEL))
    parser.add_argument("--run-time", default="10.0")
    parser.add_argument(
        "--detail", default="low", choices=["low", "medium", "high", "all", "t"]
    )
    parser.add_argument("--addr", help="use an existing server instead of spawning one")
    parser.add_argument("--binary", help="use this actr binary instead of building")
    parser.add_argument("--audit", action="store_true", help="Tcl command inventory")
    parser.add_argument("--dump-rpc", action="store_true")
    parser.add_argument("--internal", action="store_true", help=argparse.SUPPRESS)
    parser.add_argument("--gui-dir", help=argparse.SUPPRESS)
    parser.add_argument("--run-dir", help=argparse.SUPPRESS)
    return parser


def main() -> int:
    sys.stdout.reconfigure(line_buffering=True)
    args = build_parser().parse_args()
    if args.detail == "all":
        args.detail = "t"
    if args.internal:
        return run_internal(args)
    if args.audit:
        return audit(args)
    return run_parent(args)


if __name__ == "__main__":
    sys.exit(main())
