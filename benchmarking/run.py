#!/usr/bin/env python3
"""Benchmark the Lisp and Rust ACT-R engines, and the in-process Python binding.

Examples::

    python benchmarking/run.py --engine all
    python benchmarking/run.py --engine lisp --engine rust
    python benchmarking/run.py --engine rust --workload dm-scan-2000
    python benchmarking/run.py --engine lisp --connect   # use a server I started

The Lisp and Rust engines are driven by the *same* stock ``actr.py`` client over
the ACT-R RPC protocol.  The Python engine uses ``rust_actr`` in-process and
replays the same models and event counts.
"""
from __future__ import annotations

import argparse
import json
import sys
import time
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

HERE = Path(__file__).resolve().parent
ROOT = HERE.parent
sys.path.insert(0, str(HERE))

import gen_models  # noqa: E402
import harness  # noqa: E402
import report  # noqa: E402
import workloads as workload_mod  # noqa: E402

ENGINES = ("lisp", "rust", "python")


def _client_args(
    engine: str,
    selected: list[str],
    args: argparse.Namespace,
) -> list[str]:
    cmd = ["--engine", engine, "--scale", str(args.scale)]
    for name in selected:
        cmd += ["--workload", name]
    if args.repeats is not None:
        cmd += ["--repeats", str(args.repeats)]
    if args.warmup is not None:
        cmd += ["--warmup", str(args.warmup)]
    cmd += ["--overhead-calls", str(args.overhead_calls)]
    return cmd


def run_engine(
    engine: str,
    selected: list[str],
    args: argparse.Namespace,
) -> tuple[list[dict[str, Any]], dict[str, Any]]:
    client = _client_args(engine, selected, args)
    if engine == "python":
        records, meta, log = harness.run_client(client, timeout=args.timeout, cpu=args.pin_cpu)
        meta["engine"] = engine
        meta["log"] = log
        return records, meta

    if args.connect:
        records, meta, log = harness.run_client(client, timeout=args.timeout, cpu=args.pin_cpu)
        meta["engine"] = engine
        meta["log"] = log
        return records, meta

    records: list[dict[str, Any]] = []
    meta: dict[str, Any] = {}
    logs: list[str] = []

    if not args.reuse_server:
        for name in selected:
            per_workload = _client_args(engine, [name], args)
            with harness.Server(engine, timeout=args.timeout, cpu=args.pin_cpu) as server:
                recs, met, log = harness.run_client(
                    per_workload, timeout=args.timeout, cpu=args.pin_cpu
                )
            records.extend(recs)
            if not meta:
                meta = met
            logs.append(log)
    else:
        with harness.Server(engine, timeout=args.timeout, cpu=args.pin_cpu) as server:
            records, meta, log = harness.run_client(
                client, timeout=args.timeout, cpu=args.pin_cpu
            )
        logs.append(log)

    meta["engine"] = engine
    meta["log"] = "\n".join(logs)
    return records, meta


def main() -> int:
    parser = argparse.ArgumentParser(
        description=__doc__,
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    parser.add_argument(
        "--engine",
        action="append",
        choices=(*ENGINES, "all"),
        default=None,
        help="engine(s) to run (default: all)",
    )
    parser.add_argument(
        "--workload",
        action="append",
        default=[],
        help="workload name (repeatable; default: every workload)",
    )
    parser.add_argument("--list", action="store_true", help="list workloads and exit")
    parser.add_argument("--repeats", type=int, default=None)
    parser.add_argument("--warmup", type=int, default=None)
    parser.add_argument(
        "--scale",
        type=float,
        default=1.0,
        help="multiply every workload's event count (default: 1.0)",
    )
    parser.add_argument(
        "--overhead-calls",
        type=int,
        default=200,
        help="no-op RPC calls used to measure command round-trip overhead",
    )
    parser.add_argument(
        "--connect",
        action="store_true",
        help="use an ACT-R server already running instead of starting one",
    )
    parser.add_argument(
        "--reuse-server",
        action="store_true",
        help="reuse one server for all workloads instead of isolating each "
        "(the client deletes prior models before each load)",
    )
    parser.add_argument(
        "--pin-cpu",
        type=int,
        default=None,
        help="pin the server and client to one CPU with taskset (Linux)",
    )
    parser.add_argument(
        "--timeout",
        type=float,
        default=900.0,
        help="per-client timeout in seconds (default: 900)",
    )
    parser.add_argument(
        "--json-dir",
        type=Path,
        default=HERE / "results",
        help="directory for result JSON (default: benchmarking/results)",
    )
    parser.add_argument("--tag", default=None, help="optional label for the result directory")
    args = parser.parse_args()

    if args.list:
        for workload in workload_mod.WORKLOADS:
            if isinstance(workload, workload_mod.ScenarioWorkload):
                label = f"scenario:{workload.scenario}"
            else:
                label = (
                    f"{workload.events} events"
                    if workload.events is not None
                    else f"{workload.seconds}s"
                )
            print(
                f"{workload.name:24} {label:>24}  repeats={workload.repeats}  "
                f"{workload.description}"
            )
        return 0

    if args.engine is None:
        engines = list(ENGINES)
    elif "all" in args.engine:
        engines = list(ENGINES)
    else:
        engines = [engine for engine in ENGINES if engine in args.engine]

    selected = workload_mod.select(args.workload, args.scale)
    gen_models.generate()

    print(
        f"Benchmarking {', '.join(engines)} on {len(selected)} workload(s): "
        + ", ".join(workload.name for workload in selected)
    )
    print()

    all_records: list[dict[str, Any]] = []
    metas: list[dict[str, Any]] = []
    engine_data: dict[str, Any] = {}

    for engine in engines:
        print(f"[{engine}]")
        started = time.perf_counter()
        try:
            records, meta = run_engine(
                engine, [workload.name for workload in selected], args
            )
        except Exception as exc:
            print(f"  FAILED: {exc}")
            return 1
        elapsed = time.perf_counter() - started
        print(
            f"  {len(records)} workload(s) in {elapsed:.1f}s"
        )
        for record in records:
            print(
                f"  {record['workload']:24} median {report.ms(report.median_ns(record)):>10} ms"
                f"  ({record['repeats']} runs)"
            )
        all_records.extend(records)
        metas.append(meta)
        engine_data[engine] = {"meta": meta, "workloads": records}
        print()

    synthetic = [record for record in all_records if record.get("kind") != "scenario"]
    real = [record for record in all_records if record.get("kind") == "scenario"]
    synthetic_names = [
        w.name for w in selected if not isinstance(w, workload_mod.ScenarioWorkload)
    ]
    real_names = [
        w.name for w in selected if isinstance(w, workload_mod.ScenarioWorkload)
    ]

    print("=" * 78)
    if synthetic:
        print("Synthetic feature workloads")
        print(report.comparison_table(synthetic, synthetic_names))
        print()
    if real:
        print("Real-model run phase (sum of run calls per scenario)")
        print(report.scenario_table(real, "run", real_names))
        print()
        print("Real-model full scenario (includes reset/setup)")
        print(report.scenario_table(real, "scenario", real_names))
        print()
    print(report.detail_table(all_records))
    print()
    print(report.overhead_table(metas))

    stamp = datetime.now(timezone.utc).strftime("%Y%m%dT%H%M%SZ")
    name = f"{stamp}-{args.tag}" if args.tag else stamp
    out_dir = args.json_dir / name
    out_dir.mkdir(parents=True, exist_ok=True)
    summary = {
        "created": datetime.now(timezone.utc).isoformat(),
        "host": harness.metadata(),
        "scale": args.scale,
        "engines": engine_data,
        "results": all_records,
    }
    summary_path = out_dir / "summary.json"
    summary_path.write_text(json.dumps(summary, indent=2), encoding="utf-8")
    for engine, data in engine_data.items():
        (out_dir / f"{engine}.json").write_text(json.dumps(data, indent=2), encoding="utf-8")
    print()
    print(f"results: {summary_path}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
