"""Python ports of the ACT-R tutorial models.

Each module under :mod:`rust_actr.tutorial.models` exposes ``LOGICAL_PATH``
and ``build(seed)``.  The registry maps the Lisp logical path used by the
tutorial task scripts to its Python builder, so the vendored task code can run
against the Python model unchanged.
"""

from __future__ import annotations

import importlib
import pkgutil
from typing import Callable

from . import models as _models_pkg

REGISTRY: dict[str, Callable[..., object]] = {}
MODULES: dict[str, str] = {}

for _info in pkgutil.iter_modules(_models_pkg.__path__):
    _module = importlib.import_module(f"{__name__}.models.{_info.name}")
    _path = getattr(_module, "LOGICAL_PATH", None)
    _build = getattr(_module, "build", None)
    if _path and _build:
        REGISTRY[_path.upper()] = _build
        MODULES[_path.upper()] = _info.name


def build_model(logical_path: str, seed: tuple[int, int] = (100, 0)):
    """Build a tutorial model by its ACT-R logical path."""
    key = logical_path.upper()
    try:
        build = REGISTRY[key]
    except KeyError as exc:
        raise KeyError(f"no Python tutorial model for {logical_path!r}") from exc
    return build(seed=seed)


def available() -> list[str]:
    return sorted(REGISTRY)


__all__ = ["REGISTRY", "MODULES", "available", "build_model"]
