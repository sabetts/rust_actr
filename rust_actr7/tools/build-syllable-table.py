#!/usr/bin/env python3
"""Build the compact syllable-count table from ACT-R's CMU dictionary.

The engine must stay dependency-free, so the vendored `counts.lisp` is
front-coded into a small binary that `include_bytes!` embeds at compile time.
Run this from the repository root after refreshing the ACT-R checkout:

    python rust_actr7/tools/build-syllable-table.py
"""

from __future__ import annotations

import pathlib
import re
import struct

ROOT = pathlib.Path(__file__).resolve().parents[1]
SOURCE = ROOT.parent / "actr7.x" / "extras" / "syllable-count" / "counts.lisp"
OUT = ROOT / "src" / "data" / "syllable_table.bin"
LICENSE = ROOT.parent / "actr7.x" / "extras" / "syllable-count" / "cmudict_LICENSE.txt"
LICENSE_OUT = ROOT / "src" / "data" / "CMUDICT-LICENSE.txt"

MAGIC = b"ACTRSYL1"
ENTRY = re.compile(r'\("([^"]*)"\s*\.\s*(\d+)\)')


def main() -> None:
    text = SOURCE.read_text(encoding="utf-8")
    entries: dict[str, int] = {}
    for word, syllables in ENTRY.findall(text):
        entries[word.upper()] = int(syllables)

    words = sorted(entries)
    out = bytearray(MAGIC)
    out += struct.pack("<I", len(words))

    previous = b""
    for word in words:
        current = word.encode("ascii")
        prefix = 0
        limit = min(len(previous), len(current), 255)
        while prefix < limit and previous[prefix] == current[prefix]:
            prefix += 1
        suffix = current[prefix:]
        if len(suffix) > 255:
            raise SystemExit(f"word too long for the table: {word!r}")
        out.append(prefix)
        out.append(len(suffix))
        out += suffix
        out.append(entries[word])
        previous = current

    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_bytes(out)
    if LICENSE.is_file():
        LICENSE_OUT.write_bytes(LICENSE.read_bytes())

    print(
        f"wrote {len(words)} entries ({len(out)} bytes) to {OUT.relative_to(ROOT)}"
    )


if __name__ == "__main__":
    main()
