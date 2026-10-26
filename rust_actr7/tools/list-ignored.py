#!/usr/bin/env python3
"""List ignored integration-test cases and their reasons.

Covers both `ignored_case!` stubs and per-driver `#[ignore = "..."]`
annotations in `tests/cases/*.rs`. This prints one `case | reason` line per
ignored case. The dev-only `dump_spec` test in `test_models.rs` is not a case
and is not included.

Usage:
    python tools/list-ignored.py
    python tools/list-ignored.py --counts
    python tools/list-ignored.py --reason pending
    python tools/list-ignored.py --case unit5 --files
"""

import argparse
import re
import sys
from collections import Counter
from pathlib import Path

HERE = Path(__file__).resolve().parent
CASES = HERE.parent / "tests" / "cases"
CASE = re.compile(r'^//! case:\s*(\S+)', re.MULTILINE)
IGNORED_CASE = re.compile(
    r'ignored_case!\(\s*(\w+),\s*"([^"]*)",\s*"([^"]*)"\s*\);',
    re.DOTALL,
)
DIRECT_IGNORE = re.compile(r'#\[ignore(?:\s*=\s*"((?:[^"\\]|\\.)*)")?\]')


def cases() -> list[tuple[str, str, str, str]]:
    rows: list[tuple[str, str, str, str]] = []
    for path in sorted(CASES.glob("*.rs")):
        text = path.read_text(encoding="utf-8")
        header = CASE.search(text)
        name = header.group(1) if header else path.stem

        raw = text.count("ignored_case!")
        found = IGNORED_CASE.findall(text)
        if raw != len(found):
            raise SystemExit(
                f"{path.name}: parsed {len(found)} of {raw} ignored_case! calls"
            )
        for ident, case_name, reason in found:
            rows.append((case_name, reason, ident, path.name))

        for match in DIRECT_IGNORE.finditer(text):
            reason = match.group(1) if match.group(1) is not None else "(no reason)"
            rows.append((name, reason, path.stem, path.name))

    rows.sort(key=lambda r: (r[0].lower(), r[3]))
    return rows


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("--counts", action="store_true", help="group by reason and print counts")
    ap.add_argument("--reason", metavar="TEXT", help="only reasons containing TEXT")
    ap.add_argument("--case", metavar="TEXT", help="only case names or idents containing TEXT")
    ap.add_argument("--files", action="store_true", help="append the source file")
    args = ap.parse_args()

    rows = cases()
    if args.reason:
        needle = args.reason.lower()
        rows = [r for r in rows if needle in r[1].lower()]
    if args.case:
        needle = args.case.lower()
        rows = [r for r in rows if needle in r[0].lower() or needle in r[2].lower()]

    if not rows:
        print("no matching ignored cases", file=sys.stderr)
        return 1

    if args.counts:
        counts = Counter(r[1] for r in rows)
        for reason, count in sorted(counts.items(), key=lambda kv: (-kv[1], kv[0].lower())):
            print(f"{count:4}  {reason}")
    else:
        for name, reason, _ident, fname in rows:
            suffix = f"  ({fname})" if args.files else ""
            print(f"{name} | {reason}{suffix}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
