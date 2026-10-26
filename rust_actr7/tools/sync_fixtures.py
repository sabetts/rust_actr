#!/usr/bin/env python3
"""Copy or verify the vendored ACT-R fixtures.

The Rust integration tests own their oracles: `tests/cases/*.ref` and the
model/task files listed in `fixtures/models/manifest.txt`. Both are copied
verbatim from an ACT-R checkout so the crate can build and run without one.

Usage:
    python tools/sync_fixtures.py            # copy from $ACTR_DIR
    python tools/sync_fixtures.py --check    # fail if the copies differ

`ACTR_DIR` defaults to `<crate>/../actr7.x`. Model paths in the manifest use
ACT-R's logical `;` separator (e.g. `tutorial;unit1;count.lisp`).
"""

import os
import shutil
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
CRATE = HERE.parent
DEFAULT_ACTR = CRATE.parent / "actr7.x"


def actr_dir() -> Path:
    return Path(os.environ.get("ACTR_DIR", DEFAULT_ACTR))


def manifest() -> list[str]:
    path = CRATE / "fixtures" / "models" / "manifest.txt"
    out: list[str] = []
    for line in path.read_text(encoding="utf-8").splitlines():
        line = line.strip()
        if line and not line.startswith("#"):
            out.append(line)
    return out


def pairs() -> list[tuple[Path, Path]]:
    """(source under actr7.x, destination under the crate)."""
    actr = actr_dir()
    refs_src = actr / "test-models"
    refs_dst = CRATE / "tests" / "cases"
    out = [(p, refs_dst / p.name) for p in sorted(refs_src.glob("*.ref"))]
    for logical in manifest():
        rel = logical.replace(";", "/")
        out.append((actr / rel, CRATE / "fixtures" / "models" / rel))
    return out


def check() -> int:
    missing_src, missing_dst, differ = [], [], []
    for src, dst in pairs():
        if not src.is_file():
            missing_src.append(src)
        elif not dst.is_file():
            missing_dst.append(dst)
        elif src.read_bytes() != dst.read_bytes():
            differ.append(dst)
    for label, items in (
        ("missing upstream", missing_src),
        ("missing vendored", missing_dst),
        ("content differs", differ),
    ):
        for p in items:
            print(f"{label}: {p}")
    bad = len(missing_src) + len(missing_dst) + len(differ)
    print(f"checked {len(pairs())} files, {bad} problem(s)")
    return 1 if bad else 0


def copy() -> int:
    actr = actr_dir()
    if not actr.is_dir():
        print(f"ACT-R checkout not found at {actr} (set ACTR_DIR)", file=sys.stderr)
        return 1
    n = 0
    for src, dst in pairs():
        if not src.is_file():
            print(f"missing upstream: {src}", file=sys.stderr)
            return 1
        dst.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(src, dst)
        n += 1
    print(f"copied {n} files from {actr}")
    return 0


def main(argv: list[str]) -> int:
    if "--check" in argv:
        return check()
    return copy()


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
