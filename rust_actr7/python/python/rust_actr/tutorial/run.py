"""Command-line entry point for the Python tutorial models.

Examples::

    python -m rust_actr.tutorial.run --list-models
    python -m rust_actr.tutorial.run --list-tasks
    python -m rust_actr.tutorial.run demo2 experiment
    python -m rust_actr.tutorial.run fan_no_pm sentence "'lawyer'" "'store'" true person
"""

from __future__ import annotations

import argparse
import json
import sys

from . import available
from .runner import available_tasks, run_task


def _value(text: str):
    try:
        return json.loads(text)
    except json.JSONDecodeError:
        return text


def main(argv: list[str] | None = None) -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--list-models", action="store_true")
    ap.add_argument("--list-tasks", action="store_true")
    ap.add_argument("task", nargs="?")
    ap.add_argument("function", nargs="?", default="experiment")
    ap.add_argument("args", nargs="*")
    ns = ap.parse_args(argv)

    if ns.list_models:
        for path in available():
            print(path)
        return 0
    if ns.list_tasks:
        for name in available_tasks():
            print(name)
        return 0
    if not ns.task:
        ap.print_help()
        return 2
    result = run_task(ns.task, ns.function, *[_value(a) for a in ns.args])
    if result is not None:
        print(result)
    return 0


if __name__ == "__main__":
    sys.exit(main())
