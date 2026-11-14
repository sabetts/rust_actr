#!/usr/bin/env python3
"""Execute the Rust tutorial notebooks without a Jupyter dependency.

Cells tagged ``rust-actr-skip`` are not run; they are the interactive,
placeholder, or heavy cells kept for reference.

Usage:
    python tools/run_notebook.py tutorial/unit1/unit1.ipynb
    python tools/run_notebook.py --all
"""

from __future__ import annotations

import argparse
import contextlib
import io
import json
import pathlib
import sys
import traceback

ROOT = pathlib.Path(__file__).resolve().parents[1]
TUTORIAL = ROOT / "tutorial"


def cell_source(cell) -> str:
    return "".join(cell.get("source", []))


def skipped(cell) -> bool:
    tags = cell.get("metadata", {}).get("tags", [])
    return "rust-actr-skip" in tags


def run_notebook(path: pathlib.Path) -> int:
    notebook = json.loads(path.read_text(encoding="utf-8"))
    namespace = {
        "__name__": "__notebook__",
        "__file__": str(path),
    }
    ran = 0
    skipped_cells = 0
    old_cwd = pathlib.Path.cwd()
    try:
        import os

        os.chdir(path.parent)
        for index, cell in enumerate(notebook.get("cells", [])):
            if cell.get("cell_type") != "code":
                continue
            if skipped(cell):
                skipped_cells += 1
                continue
            source = cell_source(cell).strip()
            if not source:
                continue
            output = io.StringIO()
            try:
                with contextlib.redirect_stdout(output), contextlib.redirect_stderr(
                    output
                ):
                    exec(
                        compile(source, f"{path.name}:cell{index}", "exec"),
                        namespace,
                    )
            except Exception:
                print(f"FAIL {path} cell {index}")
                print(source)
                if output.getvalue():
                    print("--- output ---")
                    print(output.getvalue())
                traceback.print_exc()
                return 1
            ran += 1
    finally:
        os.chdir(old_cwd)
    print(f"OK {path.relative_to(ROOT)}: {ran} cells, {skipped_cells} skipped")
    return 0


def all_notebooks() -> list[pathlib.Path]:
    return [TUTORIAL / "tutorial.ipynb", *sorted(TUTORIAL.glob("unit*/unit*.ipynb"))]


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("notebook", nargs="*")
    parser.add_argument("--all", action="store_true")
    args = parser.parse_args()

    paths = [pathlib.Path(p) for p in args.notebook]
    if args.all:
        paths.extend(all_notebooks())
    if not paths:
        parser.error("provide one or more notebooks, or --all")

    for path in paths:
        if not path.is_absolute():
            path = (pathlib.Path.cwd() / path).resolve()
        status = run_notebook(path)
        if status:
            return status
    return 0


if __name__ == "__main__":
    sys.exit(main())
