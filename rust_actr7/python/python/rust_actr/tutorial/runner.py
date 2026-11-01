"""Run the vendored tutorial task scripts against the Python models.

The task scripts import ``actr``; this module installs :mod:`rust_actr.actr`
under that name before importing them, so ``load_act_r_model`` resolves to
the Python tutorial registry.
"""

from __future__ import annotations

import importlib
import os
import sys
from pathlib import Path
from typing import Any


def find_tutorial_dir(actr_dir: str | os.PathLike[str] | None = None) -> Path:
    """Locate ``actr7.x/tutorial/python``."""
    candidates: list[Path] = []
    root = actr_dir or os.environ.get("ACTR_DIR")
    if root:
        candidates.append(Path(root) / "tutorial" / "python")
    here = Path(__file__).resolve()
    for parent in here.parents:
        candidates.append(parent / "actr7.x" / "tutorial" / "python")
        candidates.append(parent / "tutorial" / "python")
    candidates.append(Path.cwd() / "actr7.x" / "tutorial" / "python")
    for candidate in candidates:
        if (candidate / "actr.py").is_file():
            return candidate
    raise FileNotFoundError(
        "could not find actr7.x/tutorial/python; set ACTR_DIR to the ACT-R checkout"
    )


def install_actr_module() -> Any:
    """Make ``import actr`` resolve to :mod:`rust_actr.actr`."""
    from .. import actr as local_actr

    sys.modules["actr"] = local_actr
    return local_actr


def load_task(name: str, actr_dir: str | os.PathLike[str] | None = None) -> Any:
    """Import a vendored tutorial task module (e.g. ``demo2``)."""
    tutorial_dir = find_tutorial_dir(actr_dir)
    insert = str(tutorial_dir)
    if insert not in sys.path:
        sys.path.insert(0, insert)
    install_actr_module()
    if name in sys.modules:
        module = importlib.reload(sys.modules[name])
    else:
        module = importlib.import_module(name)
    return module


def run_task(
    name: str,
    function: str = "experiment",
    *args: Any,
    actr_dir: str | os.PathLike[str] | None = None,
    **kwargs: Any,
) -> Any:
    """Load ``name`` and call ``function(*args, **kwargs)``."""
    module = load_task(name, actr_dir=actr_dir)
    target = getattr(module, function)
    return target(*args, **kwargs)


def available_tasks(actr_dir: str | os.PathLike[str] | None = None) -> list[str]:
    directory = find_tutorial_dir(actr_dir)
    return sorted(path.stem for path in directory.glob("*.py") if path.stem != "actr")
