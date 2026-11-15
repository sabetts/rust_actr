#!/usr/bin/env python3
"""Run benchmark workloads and print machine-readable results.

The same script drives:

* ``--engine lisp`` / ``--engine rust``: an already-running ACT-R RPC server,
  through the stock ``actr7.x/tutorial/python/actr.py`` client.
* ``--engine python``: the in-process ``rust_actr`` native session.

Results are emitted as one ``BENCH_JSON {...}`` line per workload, plus a
final ``BENCH_META {...}`` line.  ``run.py`` starts the servers and collects
these lines.
"""
from __future__ import annotations

import argparse
import json
import statistics
import sys
import time
import traceback
from pathlib import Path
from typing import Any

HERE = Path(__file__).resolve().parent
ROOT = HERE.parent
TUTORIAL_PYTHON = ROOT / "actr7.x" / "tutorial" / "python"
CLEAR_MODELS = HERE / "models" / "generated" / "clear-user-models.lisp"

sys.path.insert(0, str(HERE))
from workloads import Workload, select  # noqa: E402


class RpcApi:
    """The stock Python ACT-R client, talking to a server."""

    backend = "rpc"

    def __init__(self) -> None:
        sys.path.insert(0, str(TUTORIAL_PYTHON))
        import actr  # type: ignore

        self.actr = actr
        if actr.current_connection is None:
            raise RuntimeError("could not connect to an ACT-R RPC server")
        # The client enables trace monitors on connect.  Turn them off before
        # loading anything: the benchmark cares about engine time, not output.
        try:
            actr.stop_output()
        except Exception:
            pass

    def _check(self, value: Any, what: str) -> Any:
        if value is False:
            raise RuntimeError(f"{what} failed")
        return value

    def reset_models(self) -> None:
        # ACT-R auto-loads user-loads/*.lisp, and the Lisp server has no remote
        # clear-all command.  This sidecar deletes every auto-loaded model (and
        # any prior benchmark model) with `(delete-model ...)`, which both
        # servers understand.
        self._check(
            self.actr.load_act_r_code(str(CLEAR_MODELS)),
            "clear benchmark models",
        )

    def load(self, path: Path, seed: tuple[int, int]) -> None:
        self._check(self.actr.load_act_r_model(str(path)), f"load {path}")

    def set_trace(self, enabled: bool) -> None:
        # `:v` is the model-trace switch in both engines.
        self.actr.set_parameter_value(":v", enabled)

    def reset(self) -> None:
        self.actr.reset()

    def run_n_events(self, count: int) -> None:
        self._check(self.actr.run_n_events(count), "run-n-events")

    def run(self, seconds: float) -> None:
        self._check(self.actr.run(seconds), "run")

    def model_time(self) -> float:
        return float(self.actr.get_time())

    def ping(self) -> None:
        self.actr.get_time()

    def close(self) -> None:
        # `actr.stop()` assumes a socket-backed interface and fails after the
        # client switches to pipes, which both servers support.  Let the
        # process exit close the connection instead.
        pass


class LocalApi:
    """The in-process native session behind ``rust_actr``."""

    backend = "native"

    def __init__(self) -> None:
        try:
            from rust_actr import _native  # type: ignore
        except Exception as exc:  # pragma: no cover - import environment
            raise RuntimeError(
                "rust_actr is not importable; build/install it with maturin "
                "(see rust_actr7/python/README.md)"
            ) from exc
        self._native = _native
        self.session = None

    def _eval(self, name: str, *args: Any) -> list[Any]:
        if self.session is None:
            raise RuntimeError("no model loaded")
        raw = self.session.evaluate(name, json.dumps(list(args)))
        # The session accumulates trace even when it is not requested; discard
        # it so a long benchmark does not pay for unbounded trace storage.
        self.session.take_trace()
        return json.loads(raw)

    def reset_models(self) -> None:
        # A fresh Session is created for every load.
        self.session = None

    def load(self, path: Path, seed: tuple[int, int]) -> None:
        source = path.read_text(encoding="utf-8")
        self.session = self._native.Session(source, seed[0], seed[1])
        self.session.take_trace()

    def set_trace(self, enabled: bool) -> None:
        self._eval("sgp", ":v", enabled)

    def reset(self) -> None:
        self._eval("reset")

    def run_n_events(self, count: int) -> None:
        self._eval("run-n-events", count)

    def run(self, seconds: float) -> None:
        self._eval("run", seconds)

    def model_time(self) -> float:
        return float(self._eval("get-time", True)[0])

    def ping(self) -> None:
        self._eval("get-time", True)

    def close(self) -> None:
        self.session = None


def _measure(api: Any, workload: Workload, count: int | None) -> int:
    if count is not None:
        api.run_n_events(count)
    elif workload.seconds is not None:
        api.run(workload.seconds)
    else:  # pragma: no cover - registry invariant
        raise RuntimeError(f"{workload.name} has no measured operation")
    return time.perf_counter_ns()


def run_one(api: Any, workload: Workload, repeats: int, warmup: int) -> dict[str, Any]:
    count = workload.events
    path = workload.resolve_model()
    if not path.is_file():
        raise FileNotFoundError(
            f"missing benchmark model {path}; run `python gen_models.py`"
        )

    api.reset_models()
    t0 = time.perf_counter_ns()
    api.load(path, workload.seed)
    load_ns = time.perf_counter_ns() - t0

    api.set_trace(False)

    for _ in range(warmup):
        api.reset()
        _measure(api, workload, count)

    times: list[int] = []
    resets: list[int] = []
    model_times: list[float] = []
    for _ in range(repeats):
        r0 = time.perf_counter_ns()
        api.reset()
        resets.append(time.perf_counter_ns() - r0)
        t0 = time.perf_counter_ns()
        _measure(api, workload, count)
        times.append(time.perf_counter_ns() - t0)
        model_times.append(api.model_time())

    return {
        "workload": workload.name,
        "model": str(path.relative_to(HERE)),
        "events": count,
        "seconds": workload.seconds,
        "repeats": repeats,
        "warmup": warmup,
        "load_ns": load_ns,
        "reset_ns": resets,
        "times_ns": times,
        "model_times": model_times,
        "median_ns": int(statistics.median(times)),
        "min_ns": int(min(times)),
        "max_ns": int(max(times)),
        "mean_ns": int(statistics.fmean(times)),
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--engine", choices=("lisp", "rust", "python"), required=True)
    parser.add_argument("--workload", action="append", default=[])
    parser.add_argument("--repeats", type=int, default=None)
    parser.add_argument("--warmup", type=int, default=None)
    parser.add_argument("--scale", type=float, default=1.0, help="multiply event counts")
    parser.add_argument(
        "--overhead-calls",
        type=int,
        default=200,
        help="no-op RPC commands used to estimate transport overhead",
    )
    args = parser.parse_args()

    workloads = select(args.workload, args.scale)
    if not workloads:
        raise SystemExit("no workloads selected")

    api = RpcApi() if args.engine in ("lisp", "rust") else LocalApi()

    try:
        overhead: list[int] = []
        if api.backend == "rpc":
            for _ in range(args.overhead_calls):
                t0 = time.perf_counter_ns()
                api.ping()
                overhead.append(time.perf_counter_ns() - t0)
        print(
            "BENCH_META "
            + json.dumps(
                {
                    "engine": args.engine,
                    "backend": api.backend,
                    "python": sys.version.split()[0],
                    "overhead_ns": overhead,
                    "overhead_median_ns": int(statistics.median(overhead)) if overhead else None,
                }
            ),
            flush=True,
        )

        for workload in workloads:
            repeats = args.repeats if args.repeats is not None else workload.repeats
            warmup = args.warmup if args.warmup is not None else workload.warmup
            try:
                record = run_one(api, workload, repeats, warmup)
            except Exception as exc:
                print(
                    "BENCH_ERROR "
                    + json.dumps({"workload": workload.name, "error": str(exc)}),
                    flush=True,
                )
                traceback.print_exc()
                return 1
            print(
                "BENCH_JSON "
                + json.dumps({"engine": args.engine, "backend": api.backend, **record}),
                flush=True,
            )
    finally:
        api.close()
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
