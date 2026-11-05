#!/usr/bin/env python3
import argparse
import difflib
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))

from capture import Server

CASES = HERE / "cases"


def discover(selected):
    if selected:
        return [CASES / name for name in selected]
    return [path.parent for path in sorted(CASES.glob("*/client.py"))]


def normalize(proc):
    lines = [line.rstrip() for line in proc.stdout.replace("\r\n", "\n").splitlines()]
    lines = [line for line in lines if line.strip()]
    return [f"EXIT {proc.returncode}"] + lines


def first_diff(expected, actual):
    n = min(len(expected), len(actual))
    index = next((i for i in range(n) if expected[i] != actual[i]), n)
    expected_line = expected[index] if index < len(expected) else "<missing>"
    actual_line = actual[index] if index < len(actual) else "<missing>"
    return index + 1, expected_line, actual_line


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--server", choices=("sbcl", "rust", "both"), default="both")
    parser.add_argument("--timeout", type=float, default=180.0)
    parser.add_argument("--case", action="append", default=[])
    parser.add_argument("--verbose", action="store_true")
    args = parser.parse_args()

    cases = discover(args.case)
    if not cases:
        print("DIFF-FATAL no differential cases found", file=sys.stderr)
        return 2

    servers = ("sbcl", "rust") if args.server == "both" else (args.server,)
    results = {name: {} for name in servers}
    failed = False

    for name in servers:
        try:
            with Server(name, args.timeout) as srv:
                for case in cases:
                    results[name][case.name] = srv.run(
                        case / "client.py", args.timeout
                    )
        except Exception as exc:
            print(f"DIFF-FAIL {name}: {exc}")
            failed = True

    if "sbcl" in results and "rust" in results:
        for case in cases:
            name = case.name
            if name not in results["sbcl"] or name not in results["rust"]:
                print(f"DIFF-FAIL {name} missing server output")
                failed = True
                continue
            expected = normalize(results["sbcl"][name])
            actual = normalize(results["rust"][name])
            if expected == actual:
                print(f"DIFF-OK {name}")
                continue
            failed = True
            line, exp, got = first_diff(expected, actual)
            print(f"DIFF-FAIL {name} line {line}")
            print(f"  sbcl: {exp}")
            print(f"  rust: {got}")
            if args.verbose:
                for row in difflib.unified_diff(
                    expected, actual, fromfile="sbcl", tofile="rust", lineterm=""
                ):
                    print(row)

    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main())
