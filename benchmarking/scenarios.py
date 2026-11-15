#!/usr/bin/env python3
"""Real-model scenarios replayed through the benchmark command API.

Each scenario mirrors the command sequence in an existing Lisp tutorial task
or Rust test driver.  The API is implemented by both the RPC client (Lisp or
Rust server) and the in-process ``rust_actr`` session, so the same Python
sequence runs against all three backends.

A scenario returns one record per measured ``run``:

    {"reset_ns", "setup_ns", "run_ns", "model_time", "outcome"}

``run_ns`` is the engine phase used for the headline table; reset and setup
are tracked separately.
"""
from __future__ import annotations

import time
from dataclasses import dataclass
from typing import Any, Callable

Trial = dict[str, Any]


@dataclass(frozen=True)
class Scenario:
    setup: Callable[[Any, tuple[int, int]], None]
    block: Callable[[Any], list[Trial]]


def _scalar(value: Any) -> Any:
    if isinstance(value, (list, tuple)):
        return value[0] if value else None
    return value


def _text(value: Any) -> str | None:
    value = _scalar(value)
    if value is None:
        return None
    return str(value).strip()


def _reset(api: Any) -> int:
    started = time.perf_counter_ns()
    api.reset()
    return time.perf_counter_ns() - started


def _run(api: Any, seconds: float) -> tuple[int, float]:
    started = time.perf_counter_ns()
    api.run(seconds)
    return time.perf_counter_ns() - started, float(api.model_time())


def _seed_setup(api: Any, seed: tuple[int, int]) -> None:
    # `:starting-parameters` is re-applied on every reset, matching the test
    # drivers (for fan-no-pm the RNG has to restart at each trial).
    api.set_system_parameter(":starting-parameters", [":seed", list(seed)])
    api.sgp(":v", False)


# ---------------------------------------------------------------- fan-no-pm

# The test set from tutorial/lisp/fan-no-pm.lisp.
FAN_SENTENCES: tuple[tuple[str, str, bool], ...] = (
    ("lawyer", "store", True),
    ("captain", "cave", True),
    ("hippie", "church", True),
    ("debutante", "bank", True),
    ("earl", "castle", True),
    ("hippie", "bank", True),
    ("fireman", "park", True),
    ("captain", "park", True),
    ("hippie", "park", True),
    ("fireman", "store", False),
    ("captain", "store", False),
    ("giant", "store", False),
    ("fireman", "bank", False),
    ("captain", "bank", False),
    ("giant", "bank", False),
    ("lawyer", "park", False),
    ("earl", "park", False),
    ("giant", "park", False),
)


def fan_no_pm_block(api: Any) -> list[Trial]:
    trials: list[Trial] = []
    for term in ("person", "location"):
        for person, location, _target in FAN_SENTENCES:
            reset_ns = _reset(api)
            setup_started = time.perf_counter_ns()
            # Parameters the test driver sets after every reset (`fan_no_pm.rs`).
            api.sgp(":act", None)
            api.sgp(":epl", None)
            api.sgp(":pct", None)
            api.sgp(":alpha", 0.2)
            if term == "person":
                api.pdisable("retrieve-from-location")
            else:
                api.pdisable("retrieve-from-person")
            # These are Lisp strings in the model/DM, so quote them for the
            # RPC decoder (`decode-string` strips the single quotes).
            api.mod_focus("arg1", f"'{person}'", "arg2", f"'{location}'")
            setup_ns = time.perf_counter_ns() - setup_started

            run_ns, model_time = _run(api, 30.0)
            outcome = _text(api.buffer_slot_value("goal", "state"))
            trials.append(
                {
                    "reset_ns": reset_ns,
                    "setup_ns": setup_ns,
                    "run_ns": run_ns,
                    "model_time": model_time,
                    "outcome": outcome,
                }
            )
    return trials


# ---------------------------------------------------------------- semantic

SEMANTIC_CATEGORIES = ("bird", "animal", "fish")


def semantic_block(api: Any) -> list[Trial]:
    trials: list[Trial] = []
    for category in SEMANTIC_CATEGORIES:
        reset_ns = _reset(api)
        setup_started = time.perf_counter_ns()
        api.mod_focus("category", category)
        setup_ns = time.perf_counter_ns() - setup_started

        run_ns, model_time = _run(api, 10.0)
        outcome = _text(api.buffer_slot_value("goal", "judgment"))
        trials.append(
            {
                "reset_ns": reset_ns,
                "setup_ns": setup_ns,
                "run_ns": run_ns,
                "model_time": model_time,
                "outcome": outcome,
            }
        )
    return trials


# -------------------------------------------------------- semantic-dynamic

SEMANTIC_DYNAMIC_GOALS = ("g1", "g2", "g3", "g2")


def semantic_dynamic_block(api: Any) -> list[Trial]:
    # The driver does not reset between goals: the model learns/compiles
    # across them, then the next block starts from a clean reset.
    api.reset()
    trials: list[Trial] = []
    for goal in SEMANTIC_DYNAMIC_GOALS:
        setup_started = time.perf_counter_ns()
        api.goal_focus(goal)
        setup_ns = time.perf_counter_ns() - setup_started

        run_ns, model_time = _run(api, 10.0)
        outcome = _text(api.buffer_slot_value("goal", "answer"))
        trials.append(
            {
                "reset_ns": 0,
                "setup_ns": setup_ns,
                "run_ns": run_ns,
                "model_time": model_time,
                "outcome": outcome,
            }
        )
    return trials


# ----------------------------------------------------- dynamic compilation

COMPILATION_PRODUCTIONS = (
    "G1",
    "G2",
    "I1",
    "I2",
    "P1",
    "P2",
    "GD",
    "ID",
    "PD",
    "GI",
    "GP",
    "IG",
    "IP",
    "PG",
    "PI",
)


def dynamic_compilation_block(api: Any) -> list[Trial]:
    trials: list[Trial] = []
    for production in COMPILATION_PRODUCTIONS:
        reset_ns = _reset(api)
        setup_started = time.perf_counter_ns()
        api.penable(production)
        setup_ns = time.perf_counter_ns() - setup_started

        run_ns, model_time = _run(api, 0.1)
        trials.append(
            {
                "reset_ns": reset_ns,
                "setup_ns": setup_ns,
                "run_ns": run_ns,
                "model_time": model_time,
                "outcome": None,
            }
        )
    return trials


SCENARIOS: dict[str, Scenario] = {
    "fan-no-pm": Scenario(setup=_seed_setup, block=fan_no_pm_block),
    "semantic": Scenario(setup=_seed_setup, block=semantic_block),
    "semantic-dynamic": Scenario(setup=_seed_setup, block=semantic_dynamic_block),
    "dynamic-compilation": Scenario(setup=_seed_setup, block=dynamic_compilation_block),
}
