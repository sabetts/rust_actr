"""Rendering Python values as the Lisp forms used by model files."""

from __future__ import annotations

import math
from dataclasses import dataclass
from typing import Any


@dataclass(frozen=True)
class LispString:
    """A value that must be rendered as a quoted Lisp string.

    Plain Python strings are rendered as Lisp symbols, which is the usual
    case for ACT-R slots such as ``state="start"``.
    """

    value: str

    def __post_init__(self) -> None:
        if not isinstance(self.value, str):
            raise TypeError("LispString value must be a string")


String = LispString

_FORBIDDEN_SYMBOL_CHARS = set(' \t\r\n()"\';`,|')


def render_symbol(value: str, *, what: str = "symbol") -> str:
    if not isinstance(value, str):
        raise TypeError(f"{what} must be a string")
    if not value:
        raise ValueError(f"{what} must not be empty")
    bad = sorted(set(value) & _FORBIDDEN_SYMBOL_CHARS)
    if bad:
        raise ValueError(f"{what} {value!r} contains invalid character(s): {bad!r}")
    return value


def render_slot_name(value: str) -> str:
    if not isinstance(value, str):
        raise TypeError("slot name must be a string")
    return render_symbol(value.replace("_", "-"), what="slot name")


def _render_string(value: str) -> str:
    escaped = (
        value.replace("\\", "\\\\")
        .replace('"', '\\"')
        .replace("\n", "\\n")
        .replace("\t", "\\t")
    )
    return f'"{escaped}"'


def json_arg(value: Any) -> Any:
    """Coerce a Python value into the JSON argument form commands expect."""
    if isinstance(value, LispString):
        return value.value
    if value is None or isinstance(value, (str, int, float, bool)):
        return value
    if isinstance(value, (list, tuple)):
        return [json_arg(item) for item in value]
    if isinstance(value, dict):
        return {str(key): json_arg(item) for key, item in value.items()}
    raise TypeError(
        "runtime command arguments must be str, int, float, bool, None, list, tuple, "
        f"or dict; got {type(value).__name__}"
    )


def render_value(value: Any) -> str:
    """Render a Python value in the model-file Lisp dialect."""
    if isinstance(value, LispString):
        return _render_string(value.value)
    if value is None or value is False:
        return "nil"
    if value is True:
        return "t"
    if isinstance(value, int):
        return str(value)
    if isinstance(value, float):
        if not math.isfinite(value):
            raise ValueError("numeric chunk values must be finite")
        return repr(value)
    if isinstance(value, str):
        return render_symbol(value, what="symbol value")
    raise TypeError(
        "chunk values must be str, int, float, bool, None, or LispString; "
        f"got {type(value).__name__}"
    )
