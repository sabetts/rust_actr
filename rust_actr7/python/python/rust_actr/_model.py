"""A model-centric builder that renders an ACT-R ``define-model`` source."""

from __future__ import annotations

import re
from dataclasses import dataclass, field
from typing import Any, Iterable, Optional, Sequence, Union

from . import _native
from ._values import render_slot_name, render_symbol, render_value

_PRODUCTION_HEAD = re.compile(r"^\(\s*p\s+([^\s()]+)", re.IGNORECASE)


@dataclass
class Chunk:
    """A named chunk in model declarative memory."""

    name: str
    type: str
    slots: dict[str, Any] = field(default_factory=dict)

    def to_lisp(self) -> str:
        parts = [render_symbol(self.name, what="chunk name"), "isa", render_symbol(self.type)]
        for slot, value in self.slots.items():
            parts.append(slot)
            parts.append(render_value(value))
        return "(" + " ".join(parts) + ")"

    def __repr__(self) -> str:
        args = ", ".join(f"{k}={v!r}" for k, v in self.slots.items())
        return f"Chunk({self.name!r}, {self.type!r}{', ' + args if args else ''})"


@dataclass(frozen=True)
class _ChunkType:
    name: str
    slots: tuple[str, ...]
    defaults: tuple[tuple[str, Any], ...] = ()
    include: tuple[str, ...] = ()

    def to_lisp(self) -> str:
        parts = ["(chunk-type", render_symbol(self.name, what="chunk-type name")]
        for parent in self.include:
            parts.append(f"(:include {render_symbol(parent, what='parent chunk type')})")
        defaults = dict(self.defaults)
        for slot in self.slots:
            if slot in defaults:
                parts.append(f"({slot} {render_value(defaults[slot])})")
            else:
                parts.append(slot)
        return " ".join(parts) + ")"


@dataclass(frozen=True)
class _Production:
    name: str
    text: str


class Model:
    """Build and run an ACT-R model from Python.

    Definitions must be added before the first run/reset. The generated Lisp
    source is available through :meth:`to_lisp`.
    """

    def __init__(self, name: str, seed: Sequence[int] = (100, 0)) -> None:
        self.name = render_symbol(name, what="model name")
        self.seed = _check_seed(seed)
        self._chunk_types: list[_ChunkType] = []
        self._chunks: list[Chunk] = []
        self._productions: list[_Production] = []
        self._goal_focuses: list[str] = []
        self._sgp: list[tuple[str, Any]] = []
        self._engine: Optional[_native.Engine] = None

    def _check_mutable(self) -> None:
        if self._engine is not None:
            raise RuntimeError(
                "model definitions are frozen after the engine is created; "
                "create a new Model to change them"
            )

    def sgp(self, key: str, value: Any) -> "Model":
        """Add a global parameter setting (``(sgp :key value)``)."""
        self._check_mutable()
        if not isinstance(key, str) or not key.startswith(":"):
            raise ValueError("sgp key must be a string beginning with ':'")
        render_symbol(key, what="sgp key")
        self._sgp.append((key, value))
        return self

    def chunk_type(
        self,
        name: str,
        *slots: str,
        defaults: Optional[dict[str, Any]] = None,
        include: Union[str, Iterable[str]] = (),
    ) -> "Model":
        """Define a chunk type and its slots/defaults."""
        self._check_mutable()
        if len(slots) == 1 and isinstance(slots[0], (list, tuple)):
            slots = tuple(slots[0])  # type: ignore[assignment]

        type_name = render_symbol(name, what="chunk-type name")
        if any(t.name.casefold() == type_name.casefold() for t in self._chunk_types):
            raise ValueError(f"chunk type already defined: {type_name}")

        normalized_slots: list[str] = []
        for slot in slots:
            normalized = render_slot_name(slot)
            if normalized.casefold() in {s.casefold() for s in normalized_slots}:
                raise ValueError(f"duplicate slot {normalized!r} in chunk type {type_name}")
            normalized_slots.append(normalized)

        default_items = tuple(
            (render_slot_name(slot), value) for slot, value in (defaults or {}).items()
        )
        known = {s.casefold() for s in normalized_slots}
        for slot, _ in default_items:
            if slot.casefold() not in known:
                raise ValueError(
                    f"default uses slot {slot!r} not declared on chunk type {type_name}"
                )

        parents = (include,) if isinstance(include, str) else tuple(include)
        normalized_parents = tuple(
            render_symbol(parent, what="parent chunk type") for parent in parents
        )
        self._chunk_types.append(
            _ChunkType(
                type_name,
                tuple(normalized_slots),
                default_items,
                normalized_parents,
            )
        )
        return self

    def add_chunk(self, name: str, type: str, **slots: Any) -> Chunk:
        """Create a named chunk and add it to declarative memory."""
        self._check_mutable()
        chunk_name = render_symbol(name, what="chunk name")
        if any(c.name.casefold() == chunk_name.casefold() for c in self._chunks):
            raise ValueError(f"chunk already defined: {chunk_name}")
        rendered_slots = {render_slot_name(slot): value for slot, value in slots.items()}
        chunk = Chunk(chunk_name, render_symbol(type, what="chunk type"), rendered_slots)
        self._chunks.append(chunk)
        return chunk

    def goal_focus(self, chunk: Union[Chunk, str]) -> "Model":
        """Set the initial goal buffer to a chunk defined with :meth:`add_chunk`."""
        self._check_mutable()
        name = chunk.name if isinstance(chunk, Chunk) else render_symbol(chunk, what="chunk name")
        if not any(c.name.casefold() == name.casefold() for c in self._chunks):
            raise ValueError(f"goal-focus chunk is not in declarative memory: {name}")
        self._goal_focuses.append(name)
        return self

    def add_production(self, text: str) -> "Model":
        """Add a complete ``(p name ... =buffer> ... ==> ...)`` form."""
        self._check_mutable()
        if not isinstance(text, str):
            raise TypeError("production must be a Lisp source string")
        source = text.strip()
        match = _PRODUCTION_HEAD.match(source)
        if not match:
            raise ValueError("production must start with a full (p name ...) form")
        prod_name = match.group(1).upper()
        if any(p.name == prod_name for p in self._productions):
            raise ValueError(f"production already defined: {match.group(1)}")
        self._productions.append(_Production(prod_name, source))
        return self

    def to_lisp(self) -> str:
        """Render the model as a complete ``define-model`` source."""
        lines = [f"(define-model {self.name}"]
        for key, value in self._sgp:
            lines.append(f"  (sgp {key} {render_value(value)})")
        for chunk_type in self._chunk_types:
            lines.append("  " + chunk_type.to_lisp())
        for chunk in self._chunks:
            lines.append("  (add-dm " + chunk.to_lisp() + ")")
        for goal in self._goal_focuses:
            lines.append(f"  (goal-focus {goal})")
        for production in self._productions:
            lines.append(production.text)
        lines.append(")")
        return "\n".join(lines) + "\n"

    def _ensure_engine(self) -> _native.Engine:
        if self._engine is None:
            source = self.to_lisp()
            try:
                self._engine = _native.Engine(source, self.seed[0], self.seed[1])
            except Exception as exc:
                raise RuntimeError(
                    f"could not compile model {self.name!r}:\n{source}\n{exc}"
                ) from exc
        return self._engine

    def run(self, seconds: float) -> str:
        """Run for ``seconds`` of model time and return the new trace."""
        return self._ensure_engine().run(seconds)

    def run_until(self, time: float) -> str:
        """Run to an absolute model time and return the new trace."""
        return self._ensure_engine().run_until(time)

    def reset(self) -> str:
        """Reset the model and return the reset trace."""
        return self._ensure_engine().reset()

    @property
    def trace(self) -> str:
        """All trace emitted so far."""
        return self._engine.trace() if self._engine is not None else ""

    def take_trace(self) -> str:
        """Return all trace and clear the engine's trace history."""
        return self._ensure_engine().take_trace()

    def clear_trace(self) -> None:
        if self._engine is not None:
            self._engine.clear_trace()

    @property
    def time(self) -> float:
        return self._engine.time() if self._engine is not None else 0.0

    @property
    def compiled(self) -> bool:
        return self._engine is not None


def _check_seed(seed: Sequence[int]) -> tuple[int, int]:
    if not isinstance(seed, (tuple, list)) or len(seed) != 2:
        raise ValueError("seed must be a pair of non-negative integers, e.g. (100, 0)")
    a, b = seed
    if (
        isinstance(a, bool)
        or isinstance(b, bool)
        or not isinstance(a, int)
        or not isinstance(b, int)
        or a < 0
        or b < 0
    ):
        raise ValueError("seed must be a pair of non-negative integers, e.g. (100, 0)")
    return int(a), int(b)
