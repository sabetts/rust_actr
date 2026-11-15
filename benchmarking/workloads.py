#!/usr/bin/env python3
"""The workloads used by the benchmark runner.

Two kinds of workload:

* ``Workload`` — a hand-written model measured with a fixed event count
  (``production-loop``, ``dm-scan-2000``, …).  This isolates one kernel
  feature.
* ``ScenarioWorkload`` — a real model from the Lisp repo, driven through the
  same RPC command sequence a tutorial/test driver uses (``fan-no-pm``,
  ``semantic-dynamic``, …).  The scenario itself lives in ``scenarios.py``.

Both kinds replay identically over the ACT-R RPC protocol (Lisp or Rust
server) and in-process through ``rust_actr``.
"""
from __future__ import annotations

from dataclasses import dataclass, replace
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parent
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

    @property
    def kind(self) -> str:
        return "synthetic"

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


@dataclass(frozen=True)
class ScenarioWorkload:
    name: str
    model: str
    scenario: str
    seed: tuple[int, int] = (100, 0)
    repeats: int = 3
    warmup: int = 1
    description: str = ""

    @property
    def kind(self) -> str:
        return "scenario"

    def resolve_model(self) -> Path:
        path = Path(self.model)
        if path.is_absolute():
            return path
        if self.model.startswith(("actr7.x/", "benchmarking/", "rust_actr7/")):
            return (ROOT / path).resolve()
        return (MODELS / path).resolve()

    def scaled(self, scale: float) -> "ScenarioWorkload":
        if scale == 1.0:
            return self
        return replace(self, repeats=max(1, round(self.repeats * scale)))


WORKLOADS: tuple[Workload | ScenarioWorkload, ...] = (
    # ---------------------------------------------------------- synthetic
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
    # ------------------------------------------------------- real models
    ScenarioWorkload(
        name="real-fan-no-pm",
        model="actr7.x/tutorial/unit5/fan-no-pm-model.lisp",
        scenario="fan-no-pm",
        seed=(1, 0),
        repeats=3,
        description="36-trial fan retrieval experiment, no device",
    ),
    ScenarioWorkload(
        name="real-semantic",
        model="actr7.x/tutorial/unit1/semantic.lisp",
        scenario="semantic",
        seed=(100, 0),
        repeats=50,
        description="canary category queries over DM",
    ),
    ScenarioWorkload(
        name="real-semantic-dynamic",
        model="actr7.x/tutorial/unit8/semantic-dynamic-model.lisp",
        scenario="semantic-dynamic",
        seed=(100, 0),
        repeats=20,
        description="dynamic goal references + production compilation",
    ),
    ScenarioWorkload(
        name="real-dynamic-compilation",
        model="actr7.x/test-models/models/dynamic-production-compilation-tests.lisp",
        scenario="dynamic-compilation",
        seed=(100, 0),
        repeats=20,
        description="production compilation across buffer configurations",
    ),
)


def by_name(name: str) -> Workload | ScenarioWorkload:
    for workload in WORKLOADS:
        if workload.name == name:
            return workload
    raise KeyError(f"unknown workload {name!r}; choose from {', '.join(w.name for w in WORKLOADS)}")


def select(names: list[str] | None, scale: float = 1.0) -> list[Workload | ScenarioWorkload]:
    if not names:
        return [w.scaled(scale) for w in WORKLOADS]
    return [by_name(name).scaled(scale) for name in names]


def scenarios(workloads: list[Workload | ScenarioWorkload]) -> list[ScenarioWorkload]:
    return [w for w in workloads if isinstance(w, ScenarioWorkload)]
