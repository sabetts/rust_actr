#!/usr/bin/env python3
"""List ignored integration-test stubs and their reasons.

Each `tests/cases/<ident>.rs` stub calls `ignored_case!`, usually spanning
several lines. This prints one `case | reason` line per stub.

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
PATTERN = re.compile(
    r'ignored_case!\(\s*(\w+),\s*"([^"]*)",\s*"([^"]*)"\s*\);',
    re.DOTALL,
)


def stubs() -> list[tuple[str, str, str, str]]:
    rows: list[tuple[str, str, str, str]] = []
    for path in sorted(CASES.glob("*.rs")):
        text = path.read_text(encoding="utf-8")
        raw = text.count("ignored_case!")
        found = PATTERN.findall(text)
        if raw != len(found):
            raise SystemExit(
                f"{path.name}: parsed {len(found)} of {raw} ignored_case! calls"
            )
        for ident, name, reason in found:
            rows.append((name, reason, ident, path.name))
    rows.sort(key=lambda r: (r[0].lower(), r[3]))
    return rows


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("--counts", action="store_true", help="group by reason and print counts")
    ap.add_argument("--reason", metavar="TEXT", help="only reasons containing TEXT")
    ap.add_argument("--case", metavar="TEXT", help="only case names or idents containing TEXT")
    ap.add_argument("--files", action="store_true", help="append the source file")
    args = ap.parse_args()

    rows = stubs()
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
