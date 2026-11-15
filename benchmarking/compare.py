#!/usr/bin/env python3
"""Compare saved benchmark results.

    python benchmarking/compare.py benchmarking/results/20261115T210000Z
    python benchmarking/compare.py results/a/summary.json results/b/summary.json

Each path may be a summary.json, an engine json, or a directory containing
either.  The table shows median wall time per workload and the ratio to the
Lisp engine when it is present.
"""
from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from typing import Any

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import report  # noqa: E402


def records_from(path: Path) -> list[dict[str, Any]]:
    data = json.loads(path.read_text(encoding="utf-8"))
    if isinstance(data, list):
        return data
    if "results" in data:
        return list(data["results"])
    # engine json: {"meta": ..., "workloads": [...]}
    if "workloads" in data:
        engine = (data.get("meta") or {}).get("engine")
        records = list(data["workloads"])
        if engine:
            for record in records:
                record.setdefault("engine", engine)
        return records
    raise SystemExit(f"unrecognised result file: {path}")


def discover(paths: list[Path]) -> list[Path]:
    found: list[Path] = []
    for path in paths:
        if path.is_dir():
            summary = path / "summary.json"
            if summary.is_file():
                found.append(summary)
                continue
            found.extend(sorted(path.glob("*.json")))
        else:
            found.append(path)
    return found


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("paths", nargs="+", type=Path)
    parser.add_argument("--workload", action="append", default=[])
    parser.add_argument("--json", action="store_true", help="print the raw records")
    args = parser.parse_args()

    files = discover(args.paths)
    if not files:
        raise SystemExit("no result files found")

    records: list[dict[str, Any]] = []
    for path in files:
        records.extend(records_from(path))

    if args.workload:
        wanted = set(args.workload)
        records = [record for record in records if record["workload"] in wanted]

    print(report.comparison_table(records, args.workload or None))
    print()
    print(report.detail_table(records))
    if args.json:
        print()
        print(json.dumps(records, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
