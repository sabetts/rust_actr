#!/usr/bin/env python3
"""Verify that real-model scenarios do the same work in Lisp and Rust.

For each scenario this runs a single block against a fresh Lisp server and a
fresh Rust server, then compares per-trial model time and outcome.  Run this
before trusting the real-model timings.
"""
from __future__ import annotations

import argparse
import sys
from pathlib import Path
from typing import Any

HERE = Path(__file__).resolve().parent
ROOT = HERE.parent
sys.path.insert(0, str(HERE))

import gen_models  # noqa: E402
import harness  # noqa: E402
import workloads as workload_mod  # noqa: E402

ENGINES = ("lisp", "rust")


def run_scenario(engine: str, name: str, timeout: float) -> dict[str, Any]:
    args = [
        "--engine",
        engine,
        "--workload",
        name,
        "--repeats",
        "1",
        "--warmup",
        "0",
        "--overhead-calls",
        "0",
    ]
    with harness.Server(engine, timeout=timeout):
        records, _meta, _log = harness.run_client(args, timeout=timeout)
    if len(records) != 1:
        raise RuntimeError(f"{engine}/{name}: expected one record, got {len(records)}")
    return records[0]


def compare(name: str, lisp: dict[str, Any], rust: dict[str, Any]) -> list[str]:
    problems: list[str] = []
    if lisp.get("trials") != rust.get("trials"):
        problems.append(
            f"trial count: lisp={lisp.get('trials')} rust={rust.get('trials')}"
        )
    lisp_trials = lisp.get("trial_details") or []
    rust_trials = rust.get("trial_details") or []
    for index in range(min(len(lisp_trials), len(rust_trials))):
        left = lisp_trials[index]
        right = rust_trials[index]
        lisp_time = round(float(left.get("model_time", 0.0)), 6)
        rust_time = round(float(right.get("model_time", 0.0)), 6)
        if lisp_time != rust_time:
            problems.append(
                f"trial {index + 1} model time: lisp={lisp_time} rust={rust_time}"
            )
        lisp_outcome = left.get("outcome")
        rust_outcome = right.get("outcome")
        if lisp_outcome != rust_outcome:
            problems.append(
                f"trial {index + 1} outcome: lisp={lisp_outcome!r} rust={rust_outcome!r}"
            )
        if len(problems) >= 8:
            break
    return problems


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--workload", action="append", default=[])
    parser.add_argument("--timeout", type=float, default=300.0)
    args = parser.parse_args()

    gen_models.generate()
    selected = workload_mod.select(args.workload)
    scenarios = workload_mod.scenarios(selected)
    if not scenarios:
        print("no scenario workloads selected")
        return 1

    failed = False
    for workload in scenarios:
        try:
            results = {
                engine: run_scenario(engine, workload.name, args.timeout)
                for engine in ENGINES
            }
        except Exception as exc:
            print(f"VERIFY-FAIL {workload.name}: {exc}")
            failed = True
            continue
        problems = compare(workload.name, results["lisp"], results["rust"])
        if problems:
            failed = True
            print(f"VERIFY-FAIL {workload.name}")
            for problem in problems:
                print(f"  {problem}")
            continue
        trial_count = results["lisp"].get("trials", 0)
        print(
            f"VERIFY-OK   {workload.name} "
            f"({trial_count} trials, model time and outcomes match)"
        )
    return 1 if failed else 0


if __name__ == "__main__":
    raise SystemExit(main())
