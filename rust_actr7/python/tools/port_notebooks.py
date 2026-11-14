#!/usr/bin/env python3
"""Port the ACT-R tutorial notebooks to the ``rust_actr`` Python interface.

The generated notebooks live next to the ported unit modules, import those
modules instead of loading Lisp model files, and tag interactive or heavy
cells so the smoke tests can skip them.

Usage:
    python tools/port_notebooks.py [--actr DIR] [--check]
"""

from __future__ import annotations

import argparse
import json
import pathlib
import re
import sys

ROOT = pathlib.Path(__file__).resolve().parents[1]
DEFAULT_ACTR = ROOT.parents[1] / "actr7.x"
OUT = ROOT / "tutorial"

# Cells that need a human in front of an Experiment window.
HUMAN_CELLS = {
    2: {7, 17},
    3: {17, 19},
    4: {9},
    6: {9},
}

# Cells that run long experiments; they stay in the notebook but are skipped
# by the smoke tests.
HEAVY_CELLS = {
    4: {13, 23, 27, 29},
    5: {12, 20, 34, 42, 44},
    6: {13},
    7: {11, 17, 19, 35, 37, 39, 41},
    8: {15, 41, 51},
}

# Cells that are templates rather than runnable code.
PLACEHOLDER_CELLS = {
    1: {24, 26},
    5: {53, 57},
}

# The leading "reload" code cell in units 3-8 runs before any model is loaded.
TOP_RELOAD_CELLS = {3: 3, 4: 3, 5: 3, 6: 3, 7: 3, 8: 3}

# Unit 1's broken-addition section is intentionally malformed Lisp and has no
# ported module.
BROKEN_ADDITION_CELLS = set(range(19, 27))

SETUP_SOURCE = '''\
from pathlib import Path
import sys

HERE = Path.cwd()
PY_ROOT = next(
    (
        p
        for p in [HERE, *HERE.parents]
        if (p / "tutorial" / "unit1" / "count.py").is_file()
    ),
    HERE.parents[1],
)
if str(PY_ROOT) not in sys.path:
    sys.path.insert(0, str(PY_ROOT))

import rust_actr.actr as actr

actr.clear_state()
'''

RUST_NOTES = '''\
## Rust port notes

- This notebook runs on the Rust ACT-R engine through the `rust_actr` package.
- Model and task definitions live in the Python modules next to this notebook.
  Edit those modules, not the original `.lisp` files, then run `actr.reload()`.
- Cells that ask for a human participant require a live ACT-R Environment and
  are skipped in the headless Rust notebooks.
- Heavy experiment cells are kept for reference but are skipped by the automated
  smoke tests.
'''

BROKEN_ADDITION_NOTE = '''\
*Rust port:* `broken-addition.lisp` is intentionally malformed and is used to
demonstrate Lisp load warnings. It has no Python port; use the Lisp ACT-R
tutorial for this exercise.
'''


def read_json(path: pathlib.Path):
    with path.open(encoding="utf-8") as fh:
        return json.load(fh)


def write_json(nb) -> str:
    return json.dumps(nb, indent=1, ensure_ascii=False) + "\n"


def cell_source(cell) -> str:
    return "".join(cell.get("source", []))


def make_markdown(source: str):
    return {
        "cell_type": "markdown",
        "metadata": {},
        "source": source.splitlines(keepends=True),
    }


def make_code(source: str, tags=None, reason=None):
    metadata = {}
    if tags:
        metadata["tags"] = list(tags)
    if reason:
        metadata["rust_actr_skip_reason"] = reason
    return {
        "cell_type": "code",
        "execution_count": None,
        "metadata": metadata,
        "outputs": [],
        "source": source.splitlines(keepends=True),
    }


def tag_skip(cell, reason: str) -> None:
    metadata = cell.setdefault("metadata", {})
    tags = metadata.setdefault("tags", [])
    if "rust-actr-skip" not in tags:
        tags.append("rust-actr-skip")
    metadata["rust_actr_skip_reason"] = reason


def module_locations() -> dict[str, str]:
    locations: dict[str, str] = {}
    for path in sorted(OUT.glob("unit*/*.py")):
        if path.stem == "__init__":
            continue
        if path.stem in locations:
            raise SystemExit(
                f"duplicate tutorial module {path.stem!r}: "
                f"{locations[path.stem]} and {path.parent.name}"
            )
        locations[path.stem] = path.parent.name
    return locations


def model_modules() -> dict[str, tuple[str, str]]:
    mapping: dict[str, tuple[str, str]] = {}
    for path in sorted(OUT.glob("unit*/*.py")):
        text = path.read_text(encoding="utf-8")
        match = re.search(r'Python port of (ACT-R:[^"\']+?\.lisp)', text)
        if not match:
            continue
        mapping[match.group(1).lower()] = (path.parent.name, path.stem)
    return mapping


def adjust_unit_markdown(source: str) -> str:
    return re.sub(
        r"\(\.\./unit(\d+)/([^)]+)\)",
        r"(../../../../actr7.x/tutorial/unit\1/\2)",
        source,
    )


def adjust_index_markdown(source: str) -> str:
    source = re.sub(
        r"\(\./unit(\d+)/([^)]+)\)",
        r"(../../../actr7.x/tutorial/unit\1/\2)",
        source,
    )
    return re.sub(r"\(\./python/unit(\d+)\.ipynb\)", r"(./unit\1/unit\1.ipynb)", source)


def transform_code(
    source: str, unit: int, models: dict[str, tuple[str, str]], modules: dict[str, str]
) -> str:
    load = re.search(
        r"actr\.load_act_r_model\(\s*['\"](ACT-R:[^'\"]+)['\"]\s*\)", source
    )
    if load:
        logical = load.group(1)
        key = logical.lower()
        if key not in models:
            raise SystemExit(f"unit {unit}: no ported module for {logical}")
        model_unit, stem = models[key]
        if stem in {"paired_learning_model", "paired_dynamic_model"}:
            return "\n".join(
                [
                    "from tutorial.unit4 import paired",
                    f"from tutorial.{model_unit} import {stem}",
                    f"actr.use({stem}.m)",
                    f"actr.bind_module(paired.__dict__, {stem}.m)",
                ]
            )
        return "\n".join(
            [
                f"from tutorial.{model_unit} import {stem}",
                f"actr.use_module({stem})",
            ]
        )

    lines: list[str] = []
    uses: list[str] = []
    for line in source.splitlines():
        stripped = line.strip()
        if stripped == "import actr" or stripped == "import importlib":
            continue
        match = re.fullmatch(r"import ([A-Za-z_][A-Za-z0-9_]*)", stripped)
        if match and match.group(1) in modules:
            name = match.group(1)
            lines.append(f"from tutorial.{modules[name]} import {name}")
            uses.append(f"actr.use_module({name})")
            continue
        match = re.fullmatch(r"importlib\.reload\(([A-Za-z_][A-Za-z0-9_]*)\)", stripped)
        if match:
            lines.append("actr.reload()")
            continue
        lines.append(line)
    lines.extend(uses)
    return "\n".join(lines).strip()


def port_unit(unit: int, root: pathlib.Path, models, modules):
    source_path = root / "tutorial" / "python" / f"unit{unit}.ipynb"
    original = read_json(source_path)
    cells = []
    for index, cell in enumerate(original.get("cells", [])):
        if unit == 1 and index in BROKEN_ADDITION_CELLS:
            if index == min(BROKEN_ADDITION_CELLS):
                cells.append(make_markdown(BROKEN_ADDITION_NOTE))
            continue
        if TOP_RELOAD_CELLS.get(unit) == index:
            continue

        if cell.get("cell_type") == "markdown":
            source = cell_source(cell)
            if unit in TOP_RELOAD_CELLS and index == 2:
                source = (
                    "To reload after editing a ported Python model, run `actr.reload()` "
                    "from the notebook. The Rust port re-imports the model module."
                )
            cells.append(make_markdown(adjust_unit_markdown(source)))
            continue

        if cell.get("cell_type") != "code":
            continue

        reason = None
        if index in HUMAN_CELLS.get(unit, set()):
            reason = "human-mode cell requires a live ACT-R Environment"
        elif index in HEAVY_CELLS.get(unit, set()):
            reason = "heavy experiment cell skipped by smoke tests"
        elif index in PLACEHOLDER_CELLS.get(unit, set()):
            reason = "placeholder cell (edit the arguments before running)"

        source = transform_code(cell_source(cell), unit, models, modules)
        if not source.strip():
            continue
        new_cell = make_code(source)
        if reason:
            tag_skip(new_cell, reason)
        cells.append(new_cell)

    # The setup cell goes after the link/markdown header.
    insert_at = 1 if cells else 0
    cells[insert_at:insert_at] = [make_markdown(RUST_NOTES), make_code(SETUP_SOURCE)]

    notebook = dict(original)
    notebook["cells"] = cells
    notebook.setdefault("metadata", {})["rust_actr"] = {"ported": True}
    return notebook, OUT / f"unit{unit}" / f"unit{unit}.ipynb"


def port_index(root: pathlib.Path):
    source_path = root / "tutorial" / "tutorial.ipynb"
    original = read_json(source_path)
    cells = []
    for cell in original.get("cells", []):
        if cell.get("cell_type") == "markdown":
            cells.append(make_markdown(adjust_index_markdown(cell_source(cell))))
        else:
            cells.append(cell)
    notebook = dict(original)
    notebook["cells"] = cells
    notebook.setdefault("metadata", {})["rust_actr"] = {"ported": True}
    return notebook, OUT / "tutorial.ipynb"


def render(root: pathlib.Path):
    models = model_modules()
    modules = module_locations()
    rendered = {}
    for unit in range(1, 9):
        notebook, path = port_unit(unit, root, models, modules)
        rendered[path] = write_json(notebook)
    notebook, path = port_index(root)
    rendered[path] = write_json(notebook)
    return rendered


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--actr", default=str(DEFAULT_ACTR))
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()

    rendered = render(pathlib.Path(args.actr))
    if args.check:
        stale = [
            path
            for path, text in rendered.items()
            if not path.exists() or path.read_text(encoding="utf-8") != text
        ]
        for path in stale:
            print(f"stale: {path.relative_to(ROOT)}")
        return 1 if stale else 0

    for path, text in rendered.items():
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(text, encoding="utf-8")
    print(f"wrote {len(rendered)} notebooks to {OUT}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
