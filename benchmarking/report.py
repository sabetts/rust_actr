#!/usr/bin/env python3
"""Formatting and statistics for benchmark results."""
from __future__ import annotations

import statistics
from typing import Any, Iterable

ENGINE_LABELS = {"lisp": "lisp", "rust": "rust", "python": "python"}
ENGINE_ORDER = ("lisp", "rust", "python")


def median_ns(record: dict[str, Any]) -> int:
    times = record.get("times_ns") or []
    return int(statistics.median(times)) if times else int(record.get("median_ns", 0))


def median(values: list[int]) -> int:
    return int(statistics.median(values)) if values else 0


def ms(ns: int | float | None) -> str:
    if ns is None:
        return "-"
    return f"{ns / 1e6:.2f}"


def throughput(record: dict[str, Any]) -> float | None:
    """Measured operations per second (events, or seconds of model time)."""
    seconds = median_ns(record) / 1e9
    if seconds <= 0:
        return None
    if record.get("events"):
        return record["events"] / seconds
    model_times = record.get("model_times") or []
    if model_times:
        return statistics.fmean(model_times) / seconds
    return None


def index(records: Iterable[dict[str, Any]]) -> dict[str, dict[str, dict[str, Any]]]:
    out: dict[str, dict[str, dict[str, Any]]] = {}
    for record in records:
        out.setdefault(record["engine"], {})[record["workload"]] = record
    return out


def comparison_table(
    records: Iterable[dict[str, Any]],
    workloads: list[str] | None = None,
) -> str:
    records = list(records)
    data = index(records)
    engines = [name for name in ENGINE_ORDER if name in data]
    if not engines:
        return "(no results)"
    names = workloads or list(
        dict.fromkeys(record["workload"] for record in records)
    )

    header = ["workload", "events"]
    for engine in engines:
        header.append(f"{ENGINE_LABELS.get(engine, engine)} ms")
    if "lisp" in data:
        for engine in engines:
            if engine != "lisp":
                header.append(f"{engine}/lisp")

    rows: list[list[str]] = [header]
    for name in names:
        row = [name, "-"]
        for engine in engines:
            record = data.get(engine, {}).get(name)
            if record is None:
                row.append("-")
                continue
            row[1] = str(record.get("events") or f"{record.get('seconds')}s")
            row.append(ms(median_ns(record)))
        if "lisp" in data:
            base = data["lisp"].get(name)
            base_ns = median_ns(base) if base else None
            for engine in engines:
                if engine == "lisp":
                    continue
                record = data.get(engine, {}).get(name)
                if record is None or not base_ns:
                    row.append("-")
                else:
                    row.append(f"{median_ns(record) / base_ns:.2f}x")
        rows.append(row)

    widths = [max(len(row[i]) for row in rows) for i in range(len(header))]
    lines = []
    for i, row in enumerate(rows):
        line = "  ".join(cell.ljust(widths[j]) for j, cell in enumerate(row))
        lines.append(line)
        if i == 0:
            lines.append("  ".join("-" * width for width in widths))
    return "\n".join(lines)


def detail_table(records: Iterable[dict[str, Any]]) -> str:
    records = list(records)
    if not records:
        return ""
    rows: list[list[str]] = [["engine", "workload", "load ms", "reset ms"]]
    for record in records:
        reset = record.get("reset_ns") or []
        rows.append(
            [
                record["engine"],
                record["workload"],
                ms(record.get("load_ns")),
                ms(statistics.median(reset)) if reset else "-",
            ]
        )
    widths = [max(len(row[i]) for row in rows) for i in range(len(rows[0]))]
    lines = []
    for i, row in enumerate(rows):
        lines.append("  ".join(cell.ljust(widths[j]) for j, cell in enumerate(row)))
        if i == 0:
            lines.append("  ".join("-" * width for width in widths))
    return "\n".join(lines)


def scenario_table(
    records: Iterable[dict[str, Any]],
    metric: str = "run",
    workloads: list[str] | None = None,
) -> str:
    """Table for real-model scenarios.

    ``metric="run"`` uses the engine phase (sum of every ``run`` call in a
    scenario block) and adds ms/trial.  ``metric="scenario"`` uses the full
    block wall time, including resets and setup commands.
    """
    records = list(records)
    data = index(records)
    engines = [name for name in ENGINE_ORDER if name in data]
    if not engines:
        return "(no scenario results)"
    names = workloads or list(dict.fromkeys(record["workload"] for record in records))

    header = ["scenario", "trials"]
    for engine in engines:
        header.append(f"{engine} ms" if metric == "scenario" else f"{engine} run ms")
    if metric == "run":
        header.append("ms/trial")
    if "lisp" in data:
        for engine in engines:
            if engine != "lisp":
                header.append(f"{engine}/lisp")

    rows: list[list[str]] = [header]
    for name in names:
        row = [name, "-"]
        first = data[engines[0]].get(name)
        if first is not None:
            row[1] = str(first.get("trials", "-"))
        for engine in engines:
            record = data.get(engine, {}).get(name)
            if record is None:
                row.append("-")
                continue
            if metric == "scenario":
                value = median(record.get("times_ns") or [])
            else:
                value = median(record.get("run_totals_ns") or [])
            row.append(ms(value))
        if metric == "run":
            first_record = data[engines[0]].get(name)
            if first_record is not None and first_record.get("trials"):
                row.append(
                    ms(
                        median(first_record.get("run_totals_ns") or [])
                        / first_record["trials"]
                    )
                )
            else:
                row.append("-")
        if "lisp" in data:
            base = data["lisp"].get(name)
            if base is None:
                base_ns = 0
            elif metric == "scenario":
                base_ns = median(base.get("times_ns") or [])
            else:
                base_ns = median(base.get("run_totals_ns") or [])
            for engine in engines:
                if engine == "lisp":
                    continue
                record = data.get(engine, {}).get(name)
                if record is None or not base_ns:
                    row.append("-")
                    continue
                if metric == "scenario":
                    value = median(record.get("times_ns") or [])
                else:
                    value = median(record.get("run_totals_ns") or [])
                row.append(f"{value / base_ns:.2f}x")
        rows.append(row)

    widths = [max(len(row[i]) for row in rows) for i in range(len(header))]
    lines = []
    for i, row in enumerate(rows):
        lines.append("  ".join(cell.ljust(widths[j]) for j, cell in enumerate(row)))
        if i == 0:
            lines.append("  ".join("-" * width for width in widths))
    return "\n".join(lines)


def overhead_table(metas: Iterable[dict[str, Any]]) -> str:
    lines = []
    for meta in metas:
        overhead = meta.get("overhead_median_ns")
        if overhead is None:
            continue
        lines.append(
            f"{meta['engine']}: command round-trip median {overhead / 1e3:.1f} us"
            f" ({meta.get('backend')})"
        )
    return "\n".join(lines)
