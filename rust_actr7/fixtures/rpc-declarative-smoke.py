#!/usr/bin/env python3
"""Smoke test the vendored Python declarative-memory replacement.

Requires Python 3, a built `actr` binary and the ACT-R checkout. Starts no
server itself; the Rust `rpc_declarative_conformance` test starts one.
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

collected = []


def sink(text):
    collected.append(text)


actr.add_command("declarative-smoke-sink", sink)
for trace in ["model-trace", "command-trace", "warning-trace", "general-trace"]:
    actr.monitor_command(trace, "declarative-smoke-sink")

# Importing the replacement removes add-dm/dm/sdp, defines the declarative
# module, and calls extend-chunks for ct and count.
import simple_declarative  # noqa: E402,F401


def runtime_path(name):
    return os.path.join(ACTR, "tutorial", "unit1", name + ".lisp")


def load(name):
    collected.clear()
    actr.load_act_r_model(runtime_path(name))


def run(name, expected):
    result = actr.run(10)
    text = "".join(collected).lower()
    if result[1] <= 0:
        raise AssertionError("%s produced no events: %r" % (name, result))
    for token in expected:
        if token.lower() not in text:
            raise AssertionError("%s missing %r" % (name, token))


def require_int(command, chunk):
    value = actr.call_command(command, chunk)
    if isinstance(value, bool) or not isinstance(value, int):
        raise AssertionError("%s(%s) returned %r" % (command, chunk, value))
    return value


def check_dm_utilities():
    chunks = actr.dm()
    if not chunks or "ONE" not in chunks:
        raise AssertionError("python DM did not receive count chunks: %r" % (chunks,))

    ct = require_int("chunk-ct", "ONE")
    count = require_int("chunk-count", "ONE")
    if ct < 0 or count != 1:
        raise AssertionError("bad chunk parameters: ct=%r count=%r" % (ct, count))

    spec = actr.define_chunk_spec("ISA", "number", "number", "one")
    try:
        matches = actr.call_command("find-matching-chunks", spec, chunks)
    finally:
        actr.release_chunk_spec(spec)
    if not isinstance(matches, list) or "ONE" not in matches:
        raise AssertionError("find-matching-chunks missed ONE: %r" % (matches,))
    print("RPC-DECLARATIVE-OK utilities")


def check_semantic():
    load("semantic")
    answers = [
        (None, "yes"),
        (["ISA", "is-member", "object", "canary", "category", "animal"], "yes"),
        (["ISA", "is-member", "object", "canary", "category", "fish"], "no"),
    ]
    for spec, expected in answers:
        if spec is not None:
            actr.goal_focus(spec)
        result = actr.run(10)
        if result[1] <= 0:
            raise AssertionError("semantic produced no events: %r" % (result,))
        judgment = actr.buffer_slot_value("goal", "judgment")
        got = judgment.lower() if isinstance(judgment, str) else judgment
        if got != expected:
            raise AssertionError(
                "semantic judgment for %r was %r, expected %r" % (spec, got, expected)
            )
    print("RPC-DECLARATIVE-OK semantic")


try:
    load("count")
    run("count", ["PRODUCTION-FIRED STOP", "FOUR"])
    check_dm_utilities()
    print("RPC-DECLARATIVE-OK count")

    load("addition")
    run("addition", ["PRODUCTION-FIRED TERMINATE-ADDITION", "SEVEN"])
    print("RPC-DECLARATIVE-OK addition")

    check_semantic()
    print("RPC-DECLARATIVE-RESULT ok")
except AssertionError as error:
    print("RPC-DECLARATIVE-FAIL %s" % error)
    sys.exit(1)
