#!/usr/bin/env python3
"""Smoke test the vendored Python goal-module replacement against actr server.

Requires Python 3, a built `actr` binary and the ACT-R checkout. Starts no
server itself; the Rust `rpc_goal_conformance` test starts one and passes the
port by writing the usual port file through the server.
"""
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
ACTR = os.environ.get(
    "ACTR_DIR", os.path.normpath(os.path.join(HERE, "..", "..", "actr7.x"))
)
sys.path.insert(0, os.path.join(ACTR, "tutorial", "python"))
sys.path.insert(0, os.path.join(ACTR, "examples", "creating-modules", "external"))

import actr  # noqa: E402
import goal  # noqa: E402,F401

collected = []


def sink(text):
    collected.append(text)


actr.add_command("goal-smoke-sink", sink)
for trace in ["model-trace", "command-trace", "warning-trace", "general-trace"]:
    actr.monitor_command(trace, "goal-smoke-sink")


def runtime_path(name):
    return os.path.join(ACTR, "tutorial", "unit1", name + ".lisp")


def run_model(name, expected):
    collected.clear()
    actr.load_act_r_model(runtime_path(name))
    result = actr.run(10)
    text = "".join(collected).lower()
    if result[1] <= 0:
        raise AssertionError("%s produced no events: %r" % (name, result))
    for token in expected:
        if token.lower() not in text:
            raise AssertionError("%s missing %r" % (name, token))
    print("RPC-GOAL-OK %s" % name)


def run_semantic():
    collected.clear()
    actr.load_act_r_model(runtime_path("semantic"))
    runs = [
        [None],
        [["ISA", "is-member", "object", "canary", "category", "animal"]],
        [["ISA", "is-member", "object", "canary", "category", "fish"]],
    ]
    events = 0
    for spec in runs:
        if spec[0] is not None:
            actr.goal_focus(spec[0])
        result = actr.run(10)
        events += result[1]
    if events <= 0:
        raise AssertionError("semantic produced no events")
    print("RPC-GOAL-OK semantic")


try:
    run_model("count", ["PRODUCTION-FIRED STOP", "FOUR"])
    run_model("addition", ["PRODUCTION-FIRED TERMINATE-ADDITION", "SEVEN"])
    run_semantic()
    print("RPC-GOAL-RESULT ok")
except AssertionError as error:
    print("RPC-GOAL-FAIL %s" % error)
    sys.exit(1)
