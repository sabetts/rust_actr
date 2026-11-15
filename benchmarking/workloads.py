#!/usr/bin/env python3
"""The workloads used by the benchmark runner.

Each workload names a model and one measured operation.  The same workload is
replayed over the ACT-R RPC protocol (Lisp or Rust server) and in-process
through ``rust_actr``; event counts are kept identical so the three runs do
the same amount of engine work.
"""
from __future__ import annotations

from dataclasses import dataclass, replace
from pathlib import Path

HERE = Path(__file__).resolve().parent
MODELS = HERE / "models"


@dataclass(frozen=True)
class Workload:
    name: str
    model: str
    seed: tuple[int, int] = (100, 0)
    events: int | None = None
    seconds: float | None = None
    repeats: int = 3
    warmup: int = 1
    description: str = ""

    def resolve_model(self) -> Path:
        return (MODELS / self.model).resolve()

    def scaled(self, scale: float) -> "Workload":
        if scale == 1.0:
            return self
        if self.events is not None:
            return replace(self, events=max(1, round(self.events * scale)))
        if self.seconds is not None:
            return replace(self, seconds=self.seconds * scale)
        return self


WORKLOADS: tuple[Workload, ...] = (
    Workload(
        name="production-loop",
        model="production-loop.lisp",
        events=200_000,
        repeats=5,
        description="two-production toggle; scheduler + matching",
    ),
    Workload(
        name="retrieval-loop",
        model="retrieval-loop.lisp",
        events=100_000,
        repeats=5,
        description="successful retrieval every cycle",
    ),
    Workload(
        name="dm-scan-200",
        model="generated/dm-scan-200.lisp",
        events=10_000,
        repeats=5,
        description="retrieve over a 200-chunk declarative memory",
    ),
    Workload(
        name="dm-scan-2000",
        model="generated/dm-scan-2000.lisp",
        events=4_000,
        repeats=3,
        description="retrieve over a 2,000-chunk declarative memory",
    ),
    Workload(
        name="partial-matching-2000",
        model="generated/partial-matching-2000.lisp",
        events=1_000,
        repeats=5,
        description="partial match against 2,000 chunks",
    ),
)


def by_name(name: str) -> Workload:
    for workload in WORKLOADS:
        if workload.name == name:
            return workload
    raise KeyError(f"unknown workload {name!r}; choose from {', '.join(w.name for w in WORKLOADS)}")


def select(names: list[str] | None, scale: float = 1.0) -> list[Workload]:
    if not names:
        return [w.scaled(scale) for w in WORKLOADS]
    return [by_name(name).scaled(scale) for name in names]
