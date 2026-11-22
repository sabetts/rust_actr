"""A model-centric builder that renders an ACT-R ``define-model`` source.

The builder keeps the forms in call order, so a Python model reads (and
renders) like the corresponding ``.lisp`` model body.  Definition methods are
frozen once the native session is created; runtime methods added by
:class:`Model` become available at that point.
"""

from __future__ import annotations

import json
import re
from dataclasses import dataclass, field
from typing import Any, Iterable, Optional, Sequence, Union

from . import _native
from ._runtime import RuntimeMixin
from ._values import (
    LispString,
    json_arg,
    render_param_key,
    render_slot_name,
    render_symbol,
    render_value,
)

_PRODUCTION_HEAD = re.compile(r"^\(\s*p\s+([^\s()]+)", re.IGNORECASE)


@dataclass
class Chunk:
    """A named chunk.

    ``type`` is optional so sparse chunks such as ``(shark)`` can be added
    with :meth:`Model.add_dm`.
    """

    name: str
    type: Optional[str] = None
    slots: dict[str, Any] = field(default_factory=dict)

    def to_lisp(self) -> str:
        parts = [render_symbol(self.name, what="chunk name")]
        if self.type:
            parts.extend(["isa", render_symbol(self.type, what="chunk type")])
        for slot, value in self.slots.items():
            parts.append(slot)
            parts.append(render_value(value))
        return "(" + " ".join(parts) + ")"

    def __repr__(self) -> str:
        args = ", ".join(f"{k}={v!r}" for k, v in self.slots.items())
        type_part = f", {self.type!r}" if self.type else ""
        return f"Chunk({self.name!r}{type_part}{', ' + args if args else ''})"


@dataclass(frozen=True)
class _InlineChunk:
    """An unnamed chunk spec used by inline ``(goal-focus (isa ...))``."""

    type: str
    slots: dict[str, Any] = field(default_factory=dict)

    def to_lisp(self) -> str:
        parts = ["isa", render_symbol(self.type, what="chunk type")]
        for slot, value in self.slots.items():
            parts.append(slot)
            parts.append(render_value(value))
        return "(" + " ".join(parts) + ")"


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


def _render_pairs(slots: dict[str, Any]) -> str:
    parts: list[str] = []
    for slot, value in slots.items():
        parts.append(render_slot_name(slot))
        parts.append(render_value(value))
    return " ".join(parts)


def _render_similarity(entry: Sequence[Any]) -> str:
    if len(entry) != 3:
        raise ValueError("similarity entries must be (chunk_a, chunk_b, value)")
    a, b, value = entry
    return (
        f"({render_symbol(a, what='similar chunk')} "
        f"{render_symbol(b, what='similar chunk')} {render_value(value)})"
    )


def _render_base_level(entry: Sequence[Any]) -> str:
    if len(entry) not in (2, 3):
        raise ValueError("base-level entries must be (chunk, level[, creation])")
    parts = [
        render_symbol(entry[0], what="chunk name"),
        render_value(entry[1]),
    ]
    if len(entry) == 3 and entry[2] is not None:
        parts.append(render_value(entry[2]))
    return "(" + " ".join(parts) + ")"


class Model(RuntimeMixin):
    """Build and run an ACT-R model from Python.

    Definitions are added before the first run/reset and rendered by
    :meth:`to_lisp`.  The native session is created lazily on the first
    runtime call; after that, definition methods raise and the runtime
    methods inherited from :class:`RuntimeMixin` operate on the live model.
    """

    def __init__(
        self,
        name: str,
        seed: Sequence[int] = (100, 0),
        modules: Optional[Sequence[Any]] = None,
    ) -> None:
        self.name = render_symbol(name, what="model name")
        self.seed = _check_seed(seed)
        self._modules: list[Any] = list(modules or [])
        self._chunk_types: list[_ChunkType] = []
        self._chunks: list[Chunk] = []
        self._defined: list[Chunk] = []
        self._productions: list[_Production] = []
        self._goal_focuses: list[str] = []
        self._sgp: list[tuple[str, Any]] = []
        self._body: list[str] = []
        self._session: Optional[_native.Session] = None
        self._trace = ""
        self._commands: dict[str, Any] = {}
        self._monitors: dict[str, list[str]] = {}
        self._current_window: Optional[str] = None
        self._device_installed = False
        self._buffer_types: dict[str, str] = {}
        self._in_run = False

    def _check_mutable(self) -> None:
        if self._session is not None:
            raise RuntimeError(
                "model definitions are frozen after the engine is created; "
                "create a new Model to change them"
            )

    def _emit(self, form: str) -> None:
        self._body.append(form)

    def add_module(self, module: Any) -> None:
        """Register a module class/factory before the engine is built."""
        self._check_mutable()
        self._modules.append(module)

    def _ensure_session(self) -> _native.Session:
        if self._session is None:
            source = self.to_lisp()
            try:
                self._session = _native.Session(
                    source,
                    self.seed[0],
                    self.seed[1],
                    modules=self._modules,
                )
            except Exception as exc:
                raise RuntimeError(
                    f"could not compile model {self.name!r}:\n{source}\n{exc}"
                ) from exc
        return self._session

    def _eval(self, name: str, *args: Any) -> list[Any]:
        session = self._ensure_session()
        payload = json.dumps([json_arg(arg) for arg in args])
        try:
            raw = session.evaluate(name, payload)
        except Exception as exc:
            raise RuntimeError(f"command {name!r} failed: {exc}") from exc
        result = json.loads(raw)
        text = session.take_trace()
        if text:
            self._trace += text
            self._deliver_trace(text)
        self._dispatch_fired()
        return result

    def _needs_stepping(self) -> bool:
        """Whether a run must yield to Python between engine events."""
        return bool(self._monitors or self._commands)

    def _run_result(self, name: str, *args: Any) -> list[Any]:
        if (
            name in ("run", "run-full-time", "run-until-time")
            and args
            and self._needs_stepping()
        ):
            return self._run_stepped_result(name, args[0])
        return self._eval(name, *args)

    def _run_stepped_result(self, name: str, value: Any) -> list[Any]:
        session = self._ensure_session()
        if self._in_run:
            raise RuntimeError("run() cannot be called from inside a task callback")
        self._in_run = True
        try:
            limit, full, should_run = session.run_begin(name, value)
            if not should_run:
                return [session.time(), session.run_events(), None]
            while True:
                more = session.run_chunk(limit, full, 1, None)
                self._absorb_trace()
                handled = self._complete_host_action()
                handled = self._dispatch_device_actions() or handled
                self._dispatch_fired()
                if not more and not handled:
                    break
            return [session.time(), session.run_events(), None]
        finally:
            self._in_run = False

    def _absorb_trace(self) -> None:
        text = self._session.take_trace() if self._session is not None else ""
        if text:
            self._trace += text
            self._deliver_trace(text)

    def _dispatch_device_actions(self) -> bool:
        if self._session is None:
            return False
        handled = False
        for command, args_json in self._session.take_pending_device_actions():
            fn = self._commands.get(command.upper())
            if fn is not None:
                fn(*json.loads(args_json))
                handled = True
        return handled

    def _complete_host_action(self) -> bool:
        if self._session is None:
            return False
        pending = self._session.take_pending_host_action()
        if pending is None:
            return False
        name, args, simple = pending
        fn = self._commands.get(name.upper())
        result = None
        if fn is not None:
            try:
                result = fn(*args)
            except Exception:
                result = None
        self._session.complete_host_action(_action_result_text(result), simple)
        return True

    def _dispatch_fired(self) -> None:
        if self._session is None:
            return
        for command, model, key in self._session.take_fired_commands():
            self._fire(command, [model, key])
        for command, args in self._session.take_fired_task_commands():
            values = list(args)
            fn = self._commands.get(command.upper())
            if fn is not None:
                fn(*values)
            self._fire(command, values)

    def _fire(self, command: str, args: list[Any]) -> None:
        for monitor in self._monitors.get(command.upper(), []):
            fn = self._commands.get(monitor.upper())
            if fn is not None:
                fn(*args)

    def _deliver_trace(self, text: str) -> None:
        monitors = self._monitors.get("MODEL-TRACE", [])
        if not monitors:
            return
        for monitor in monitors:
            fn = self._commands.get(monitor.upper())
            if fn is not None:
                for line in text.splitlines():
                    fn(line)

    def sgp(self, key: str, value: Any) -> "Model":
        """Set a global parameter (``(sgp :key value)``)."""
        key = render_param_key(key)
        render_symbol(key, what="sgp key")
        self._sgp.append((key, value))
        if self._session is not None:
            self._eval("sgp", key, value)
            return self
        self._check_mutable()
        self._emit(f"(sgp {key} {render_value(value)})")
        return self

    @property
    def sim_hook_name(self) -> Optional[str]:
        """The model's ``:sim-hook`` function name, if any."""
        for key, value in self._sgp:
            if key.upper() == ":SIM-HOOK":
                if isinstance(value, LispString):
                    return value.value
                return str(value)
        return None

    def set_sim_hook(self, callable: Any) -> "Model":
        """Install a pure Python similarity hook for this model."""
        self._ensure_session().set_sim_hook(callable)
        return self

    @property
    def ppm_hook_name(self) -> Optional[str]:
        """The model's ``:ppm-hook`` function name, if any."""
        for key, value in self._sgp:
            if key.upper() == ":PPM-HOOK":
                if isinstance(value, LispString):
                    return value.value
                return str(value)
        return None

    def set_ppm_hook(self, callable: Any) -> "Model":
        """Install a pure Python procedural partial-matching hook."""
        self._ensure_session().set_ppm_hook(callable)
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
        chunk_type = _ChunkType(
            type_name,
            tuple(normalized_slots),
            default_items,
            normalized_parents,
        )
        self._chunk_types.append(chunk_type)
        self._emit(chunk_type.to_lisp())
        return self

    def add_chunk(
        self,
        name: str,
        type: Optional[str] = None,
        **slots: Any,
    ) -> Chunk:
        """Create a named chunk and add it to declarative memory.

        ``type`` may be omitted for sparse chunks, e.g.
        ``m.add_chunk("shark")`` renders ``(shark)``.
        """
        self._check_mutable()
        chunk_name = render_symbol(name, what="chunk name")
        if any(c.name.casefold() == chunk_name.casefold() for c in self._chunks):
            raise ValueError(f"chunk already defined: {chunk_name}")
        rendered_slots = {render_slot_name(slot): value for slot, value in slots.items()}
        chunk = Chunk(chunk_name, render_symbol(type, what="chunk type") if type else None, rendered_slots)
        self._chunks.append(chunk)
        self._emit("(add-dm " + chunk.to_lisp() + ")")
        return chunk

    def add_dm(self, *chunks: Union[Chunk, str]) -> "Model":
        """Add bare/sparse chunks to declarative memory.

        Accepts named :class:`Chunk` objects or strings.  For typed chunks use
        :meth:`add_chunk` directly.
        """
        if self._session is not None:
            for item in _runtime_chunk_items(chunks):
                self._eval("add-dm", _chunk_json(item))
            return self
        self._check_mutable()
        for item in chunks:
            if isinstance(item, Chunk):
                chunk = item
            elif isinstance(item, str):
                chunk = Chunk(render_symbol(item, what="chunk name"))
            else:
                raise TypeError("add_dm accepts Chunk objects or chunk names")
            if any(c.name.casefold() == chunk.name.casefold() for c in self._chunks):
                raise ValueError(f"chunk already defined: {chunk.name}")
            self._chunks.append(chunk)
            self._emit("(add-dm " + chunk.to_lisp() + ")")
        return self

    def define_chunks(self, *chunks: Union[Chunk, str]) -> "Model":
        """Define named chunks without adding them to declarative memory."""
        if self._session is not None:
            for item in _runtime_chunk_items(chunks):
                self._eval("define-chunks", _chunk_json(item))
            return self
        self._check_mutable()
        for item in chunks:
            if isinstance(item, Chunk):
                chunk = item
            elif isinstance(item, str):
                chunk = Chunk(render_symbol(item, what="chunk name"))
            else:
                raise TypeError("define_chunks accepts Chunk objects or chunk names")
            if any(c.name.casefold() == chunk.name.casefold() for c in self._defined):
                raise ValueError(f"chunk already defined: {chunk.name}")
            self._defined.append(chunk)
            self._emit("(define-chunks " + chunk.to_lisp() + ")")
        return self

    def goal_focus(self, chunk: Union[Chunk, str, None] = None, **slots: Any) -> "Model":
        """Set the initial goal buffer.

        Pass a :class:`Chunk`/name from declarative memory, or use
        ``m.goal_focus(isa="goal", state="start")`` for an inline chunk.
        """
        if self._session is not None or isinstance(chunk, (list, tuple)):
            if chunk is not None and slots:
                raise TypeError("goal_focus accepts either a chunk or keyword slots, not both")
            if chunk is None:
                isa = slots.pop("isa", None)
                if isa is None:
                    raise TypeError("goal_focus requires a chunk or isa=...")
                spec: Any = ["isa", isa]
                for slot, value in slots.items():
                    spec.extend([render_slot_name(slot), value])
            elif isinstance(chunk, (list, tuple)):
                spec = list(chunk)
                if spec and not any(str(x).lower() == "isa" for x in spec):
                    typ = self._buffer_types.get("GOAL")
                    if typ:
                        spec = ["isa", typ, *spec]
            elif isinstance(chunk, Chunk):
                spec = chunk.name
            else:
                spec = chunk
            self._eval("goal-focus", spec)
            return self
        self._check_mutable()
        if chunk is not None and slots:
            raise TypeError("goal_focus accepts either a chunk or keyword slots, not both")
        if chunk is None:
            isa = slots.pop("isa", None)
            if isa is None:
                raise TypeError("goal_focus requires a chunk or isa=...")
            inline = _InlineChunk(
                render_symbol(isa, what="chunk type"),
                {render_slot_name(slot): value for slot, value in slots.items()},
            )
            self._goal_focuses.append(inline.to_lisp())
            self._emit("(goal-focus " + inline.to_lisp() + ")")
            return self
        name = chunk.name if isinstance(chunk, Chunk) else render_symbol(chunk, what="chunk name")
        if not any(c.name.casefold() == name.casefold() for c in self._chunks):
            raise ValueError(f"goal-focus chunk is not in declarative memory: {name}")
        self._goal_focuses.append(name)
        self._emit(f"(goal-focus {name})")
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
        self._emit(source)
        return self

    def spp(self, production: str, **params: Any) -> "Model":
        """Set production parameters (``(spp <production> :key value)``)."""
        name = render_symbol(production, what="production name")
        if not params:
            raise ValueError("spp requires at least one parameter")
        if self._session is not None:
            flat: list[Any] = [name]
            for key, value in params.items():
                flat.extend([render_param_key(key), value])
            self._eval("spp", *flat)
            return self
        self._check_mutable()
        rendered = " ".join(
            f"{render_param_key(key)} {render_value(value)}"
            for key, value in params.items()
        )
        self._emit(f"(spp {name} {rendered})")
        return self

    def set_similarities(self, *pairs: Sequence[Any]) -> "Model":
        """Set explicit similarities between chunks.

        Accepts ``(a, b, value)`` sequences or a single iterable of them.
        """
        if self._session is not None:
            return RuntimeMixin.set_similarities(self, *pairs)  # type: ignore[return-value]
        self._check_mutable()
        entries = _as_entries(pairs, 3, "similarity")
        rendered = " ".join(_render_similarity(entry) for entry in entries)
        self._emit(f"(set-similarities {rendered})")
        return self

    def set_base_levels(self, *pairs: Sequence[Any]) -> "Model":
        """Set fixed base levels for named chunks."""
        if self._session is not None:
            return RuntimeMixin.set_base_levels(self, *pairs)  # type: ignore[return-value]
        self._check_mutable()
        entries = _as_entries(pairs, (2, 3), "base-level")
        rendered = " ".join(_render_base_level(entry) for entry in entries)
        self._emit(f"(set-base-levels {rendered})")
        return self

    def set_all_base_levels(self, level: Any, creation: Any = None) -> "Model":
        """Set a fixed base level for every chunk in declarative memory."""
        if self._session is not None:
            RuntimeMixin.set_all_base_levels(self, level, creation)
            return self
        self._check_mutable()
        args = [render_value(level)]
        if creation is not None:
            args.append(render_value(creation))
        self._emit("(set-all-base-levels " + " ".join(args) + ")")
        return self

    def declare_buffer_usage(
        self,
        buffer: str,
        chunk_type: str,
        *slots: str,
        all_slots: bool = False,
    ) -> "Model":
        """Declare the chunk type a buffer is used with.

        Pass ``all_slots=True`` for ACT-R's ``:all`` keyword.
        """
        self._check_mutable()
        buffer_name = render_symbol(buffer, what="buffer name")
        type_name = render_symbol(chunk_type, what="chunk type")
        self._buffer_types[buffer_name.upper()] = type_name.upper()
        parts = [buffer_name, type_name]
        rendered_slots = [render_slot_name(slot) for slot in slots]
        parts.extend(rendered_slots)
        if all_slots and ":all" not in {slot.lower() for slot in rendered_slots}:
            parts.append(":all")
        self._emit("(declare-buffer-usage " + " ".join(parts) + ")")
        return self

    def set_visloc_default(self, **slots: Any) -> "Model":
        """Set default constraints for an unconstrained visual-location request."""
        self._check_mutable()
        if not slots:
            raise ValueError("set_visloc_default requires at least one slot")
        self._emit("(set-visloc-default " + _render_pairs(slots) + ")")
        return self

    def call_act_r_command(self, name: str, *args: Any) -> "Model":
        """Schedule a task command from the model body."""
        self._check_mutable()
        command = render_symbol(name, what="command name")
        rendered = " ".join(render_value(arg) for arg in args)
        suffix = f" {rendered}" if rendered else ""
        self._emit(f'(call-act-r-command "{command}"{suffix})')
        return self

    def set_initial_buffer(
        self,
        buffer: str,
        chunk: Union[Chunk, str, None] = None,
        **slots: Any,
    ) -> "Model":
        """Set a buffer's initial chunk in the model body (re-applied on reset)."""
        self._check_mutable()
        buffer_name = render_symbol(buffer, what="buffer name")
        if chunk is not None and slots:
            raise TypeError("set_initial_buffer accepts either a chunk or keyword slots")
        if chunk is None:
            isa = slots.pop("isa", None)
            if isa is None:
                raise TypeError("set_initial_buffer requires a chunk or isa=...")
            spec = _InlineChunk(
                render_symbol(isa, what="chunk type"),
                {render_slot_name(slot): value for slot, value in slots.items()},
            ).to_lisp()
        elif isinstance(chunk, Chunk):
            spec = chunk.to_lisp()
        else:
            spec = render_symbol(chunk, what="chunk name")
        self._emit(f"(set-buffer-chunk '{buffer_name} '{spec})")
        return self

    def install_device(
        self,
        window: Union[str, bool] = True,
        **options: Any,
    ) -> "Model":
        """Install a device in the model body (before the first run).

        ``window`` may be a window title or ``True`` for the default title.
        """
        if self._session is not None:
            return RuntimeMixin.install_device(self, window)
        self._check_mutable()
        title = "window" if window is True else (window if isinstance(window, str) else None)
        if title is None:
            raise TypeError("install_device window must be a title or True")
        opts = _render_window_options(options)
        suffix = f" {opts}" if opts else ""
        title_text = render_value(LispString(title))
        self._emit(f"(install-device (open-exp-window {title_text}{suffix}))")
        return self

    def form(self, source: str) -> "Model":
        """Append a raw model-body form."""
        self._check_mutable()
        if not isinstance(source, str) or not source.strip():
            raise TypeError("form requires a non-empty Lisp source string")
        self._emit(source.strip())
        return self

    def to_lisp(self) -> str:
        """Render the model as a complete ``define-model`` source."""
        lines = [f"(define-model {self.name}"]
        lines.extend("  " + form for form in self._body)
        lines.append(")")
        return "\n".join(lines) + "\n"

def _action_result_text(value: Any) -> Optional[str]:
    if value is None:
        return None
    if isinstance(value, str):
        return value
    if isinstance(value, (list, tuple)):
        return "(" + " ".join(str(item) for item in value) + ")"
    return str(value)


def _runtime_chunk_items(chunks: Sequence[Any]) -> list[Any]:
    """Flatten the two tutorial runtime shapes: ``f(x)`` and ``f([x, y])``."""
    if len(chunks) == 1 and isinstance(chunks[0], (list, tuple)):
        items = list(chunks[0])
        if items and all(isinstance(item, (str, Chunk)) for item in items):
            return items
        if items and all(isinstance(item, (list, tuple, Chunk)) for item in items):
            return items
    return list(chunks)


def _chunk_json(chunk: Union[Chunk, str, list, tuple]) -> Any:
    if isinstance(chunk, (list, tuple)):
        return list(chunk)
    if isinstance(chunk, str):
        return [render_symbol(chunk, what="chunk name")]
    if not isinstance(chunk, Chunk):
        raise TypeError("chunks must be Chunk objects or chunk names")
    spec: list[Any] = [chunk.name]
    if chunk.type:
        spec.extend(["isa", chunk.type])
    for slot, value in chunk.slots.items():
        spec.extend([slot, json_arg(value)])
    return spec


def _as_entries(
    pairs: Sequence[Sequence[Any]],
    width: Union[int, Sequence[int]],
    what: str,
) -> list[Sequence[Any]]:
    """Normalize ``(a, b, v)`` args or a single iterable of entries."""
    if not pairs:
        raise ValueError(f"{what} requires at least one entry")
    widths = (width,) if isinstance(width, int) else tuple(width)
    if len(pairs) == 1 and isinstance(pairs[0], (list, tuple)):
        candidate = pairs[0]
        scalar_entry = len(candidate) in widths and all(
            not isinstance(item, (list, tuple)) for item in candidate
        )
        entries = [candidate] if scalar_entry else list(candidate)
    else:
        entries = list(pairs)
    for entry in entries:
        if not isinstance(entry, (list, tuple)) or len(entry) not in widths:
            raise ValueError(f"invalid {what} entry: {entry!r}")
    return entries


def _render_window_options(options: dict[str, Any]) -> str:
    parts: list[str] = []
    for key, value in options.items():
        parts.append(":" + key.replace("_", "-"))
        parts.append(render_value(value))
    return " ".join(parts)


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
