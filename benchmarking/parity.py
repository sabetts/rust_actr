#!/usr/bin/env python3
"""Check that every benchmark model traces identically in Lisp and Rust.

The run uses each engine's direct model runner (not the RPC dispatcher), so it
tests the model and the kernel rather than the transport:

* Rust: ``actr run --run <seconds> <model>``
* Lisp: a temporary SBCL script that loads ACT-R, deletes the auto-loaded user
  models, loads the model, and calls ``(run <seconds>)``.

Each model gets a temporary copy with ``:v t`` so the trace is enabled.  Run
this after changing a benchmark model.
"""
from __future__ import annotations

import argparse
import re
import subprocess
import sys
import tempfile
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parent
sys.path.insert(0, str(HERE))

import harness  # noqa: E402
ACTR = ROOT / "actr7.x"
RUST = ROOT / "rust_actr7" / "target" / "release" / "actr"
CLEAR = HERE / "models" / "generated" / "clear-user-models.lisp"
TRACE_LINE = re.compile(r"^\s*\d+\.\d+\s+\S")


def trace_lines(text: str) -> list[str]:
    return [
        line.rstrip()
        for line in text.replace("\r\n", "\n").splitlines()
        if TRACE_LINE.match(line)
    ]


def trace_copy(model: Path, directory: Path) -> Path:
    text = model.read_text(encoding="utf-8")
    if ":v nil" not in text:
        raise RuntimeError(f"{model} does not contain ':v nil'")
    path = directory / model.name
    path.write_text(text.replace(":v nil", ":v t", 1), encoding="utf-8")
    return path


def run_rust(model: Path, seconds: float, timeout: float) -> list[str]:
    if not RUST.is_file():
        raise RuntimeError(f"missing release binary {RUST}")
    proc = subprocess.run(
        [str(RUST), "run", "--run", str(seconds), str(model)],
        cwd=ROOT / "rust_actr7",
        capture_output=True,
        text=True,
        timeout=timeout,
    )
    if proc.returncode != 0:
        raise RuntimeError(f"rust run failed:\n{proc.stdout}\n{proc.stderr}")
    return trace_lines(proc.stdout)


def run_lisp(model: Path, seconds: float, timeout: float) -> list[str]:
    with tempfile.TemporaryDirectory() as tmp:
        script = Path(tmp) / "run.lisp"
        script.write_text(
            "\n".join(
                [
                    f'(load "{ACTR / "load-act-r.lisp"}")',
                    f'(load "{CLEAR}")',
                    f'(load "{model}")',
                    f"(run {seconds})",
                ]
            ),
            encoding="utf-8",
        )
        proc = subprocess.run(
            [
                "sbcl",
                "--noinform",
                "--non-interactive",
                "--load",
                str(script),
            ],
            cwd=ACTR,
            capture_output=True,
            text=True,
            timeout=timeout,
        )
    if proc.returncode != 0:
        raise RuntimeError(f"lisp run failed:\n{proc.stdout}\n{proc.stderr}")
    return trace_lines(proc.stdout)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--workload", action="append", default=[])
    parser.add_argument("--seconds", type=float, default=1.0)
    parser.add_argument("--timeout", type=float, default=180.0)
    parser.add_argument("--verbose", action="store_true")
    args = parser.parse_args()

    import gen_models
    import workloads as workload_mod

    gen_models.generate()
    selected = workload_mod.select(args.workload)
    failed = False
    saved = harness.save_actr_files()

    try:
        with tempfile.TemporaryDirectory() as tmp:
            directory = Path(tmp)
            for workload in selected:
                model = trace_copy(workload.resolve_model(), directory)
                try:
                    lisp = run_lisp(model, args.seconds, args.timeout)
                    rust = run_rust(model, args.seconds, args.timeout)
                except Exception as exc:
                    print(f"PARITY-FAIL {workload.name}: {exc}")
                    failed = True
                    continue
                if lisp == rust:
                    print(f"PARITY-OK   {workload.name} ({len(lisp)} trace lines)")
                    if args.verbose:
                        for line in lisp:
                            print("  " + line)
                    continue
                failed = True
                print(f"PARITY-FAIL {workload.name}")
                for index in range(max(len(lisp), len(rust))):
                    left = lisp[index] if index < len(lisp) else "<missing>"
                    right = rust[index] if index < len(rust) else "<missing>"
                    if left != right:
                        print(f"  line {index + 1}:")
                        print(f"    lisp: {left}")
                        print(f"    rust: {right}")
                        break
    finally:
        harness.restore_actr_files(saved)
    return 1 if failed else 0


if __name__ == "__main__":
    raise SystemExit(main())
