#!/usr/bin/env python3
"""RPC conformance driver: drive the vendored actr.py client against a
running `actr-server` and compare each Python test-model's output to the
shared `.ref` files.

Usage:
  rpc-conformance.py <port> [test ...]        # run the given tests
  rpc-conformance.py <port> --all             # run every test-model with a ref
  rpc-conformance.py <port> --expect <file>   # assert the named tests pass

Exit status is non-zero if any requested test that is expected to pass does
not. Output is one `RPC-OK <name>` / `RPC-FAIL <name> <detail>` line per test.
"""
import importlib
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
ACTR = os.environ.get("ACTR_DIR", os.path.normpath(os.path.join(HERE, "..", "..", "actr7.x")))
sys.path.insert(0, os.path.join(ACTR, "tutorial", "python"))
sys.path.insert(0, os.path.join(ACTR, "test-models"))

import actr  # noqa: E402  (auto-connects from ~/act-r-port-num.txt)


def all_tests():
    d = os.path.join(ACTR, "test-models")
    names = []
    for f in sorted(os.listdir(d)):
        if f.endswith(".py"):
            n = f[:-3]
            if os.path.isfile(os.path.join(d, n + ".ref")):
                names.append(n)
    return names


def normalise(chunks):
    """Flatten captured trace+stdout chunks into comparable lines."""
    return [l.rstrip().lower() for l in "".join(chunks).splitlines() if l.strip()]


class _Collector:
    """A `tester.out_file`: task `print`s are appended raw to the buffer so
    they interleave with the monitored trace in arrival order."""

    def __init__(self, buffer):
        self._b = buffer

    def write(self, s):
        self._b.append(s)

    def flush(self):
        pass


def uses_tester(name):
    path = os.path.join(ACTR, "test-models", name + ".py")
    try:
        with open(path) as f:
            return "import tester" in f.read()
    except OSError:
        return False


def run_test(name, collected):
    """Run one test-model, capturing its trace and `tester.out_file` output."""
    collected.clear()
    if uses_tester(name):
        try:
            import tester  # noqa: F401

            tester.out_file = _Collector(collected)
        except Exception:
            pass
    mod = importlib.import_module(name)
    importlib.reload(mod)
    mod.run_it()
    return list(collected)


def main():
    if len(sys.argv) < 2:
        print(__doc__)
        return 2
    port = sys.argv[1]
    args = sys.argv[2:]

    expect = None
    if args and args[0] == "--expect":
        expect = [l.strip() for l in open(args[1]) if l.strip() and not l.startswith("#")]
        tests = list(expect)
    elif args and args[0] == "--all":
        tests = all_tests()
    else:
        tests = args or all_tests()

    if actr.current_connection is None:
        print("RPC-FATAL could not connect on port", port, file=sys.stderr)
        return 2

    collected = []

    def sink(s):
        collected.append(s if s.endswith("\n") else s + "\n")

    actr.add_command("rpc-sink", sink)
    for c in ["model-trace", "command-trace", "warning-trace", "general-trace"]:
        actr.monitor_command(c, "rpc-sink")

    failed = 0
    for name in tests:
        try:
            got = normalise(run_test(name, collected))
        except Exception as e:  # a driver/harness error, not an engine mismatch
            print("RPC-FAIL %s (driver error: %s)" % (name, e))
            if expect is not None and name in expect:
                failed += 1
            continue
        ref_path = os.path.join(ACTR, "test-models", name + ".ref")
        exp = normalise([open(ref_path).read()])
        if got == exp:
            print("RPC-OK %s" % name)
        else:
            n = min(len(got), len(exp))
            first = next((i for i in range(n) if got[i] != exp[i]), n)
            detail = "line %d/%d/%d" % (first + 1, len(got), len(exp))
            print("RPC-FAIL %s (%s)" % (name, detail))
            if expect is not None and name in expect:
                failed += 1

    for c in ["model-trace", "command-trace", "warning-trace", "general-trace"]:
        actr.remove_command_monitor(c, "rpc-sink")

    if expect is not None:
        if failed:
            print("RPC-RESULT expected-pass failures: %d" % failed)
            return 1
        print("RPC-RESULT %d expected tests passed" % len(expect))
    return 0


if __name__ == "__main__":
    sys.exit(main())
