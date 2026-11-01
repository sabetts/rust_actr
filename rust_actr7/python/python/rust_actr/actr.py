"""A module-level façade matching the tutorial ``actr`` client.

The vendored tutorial task scripts call functions such as ``actr.run`` and
``actr.load_act_r_model``.  This module binds those calls to the current
:class:`rust_actr.Model`; ``load_act_r_model`` resolves the logical path
through :mod:`rust_actr.tutorial` instead of loading Lisp.

Usage::

    from rust_actr import actr
    actr.load_act_r_model("ACT-R:tutorial;unit1;count.lisp")
    print(actr.run(10))
"""

from __future__ import annotations

from typing import Any, Optional

from .tutorial import REGISTRY, build_model

_current: Optional[Any] = None
_current_path: Optional[str] = None
_pending_commands: dict[str, Any] = {}
_pending_monitors: list[tuple[str, str]] = []


def use(model: Any) -> Any:
    """Bind ``model`` as the current model."""
    global _current, _current_path
    _current = model
    _current_path = None
    return model


def current_model() -> Optional[Any]:
    return _current


def load_act_r_model(path: str) -> Any:
    """Build the Python model registered for ``path`` and make it current."""
    global _current, _current_path
    key = path.upper()
    if key not in REGISTRY:
        raise KeyError(f"no Python tutorial model for {path!r}")
    _current = build_model(path)
    _current_path = path
    for name, function in _pending_commands.items():
        _current.add_command(name, function)
    for original, monitor in _pending_monitors:
        _current.monitor_command(original, monitor)
    return _current


def add_command(
    name: str,
    function: Any = None,
    documentation: str = "No documentation provided.",
    single: bool = True,
    local_name: Optional[str] = None,
    encoded: bool = False,
) -> Any:
    if _current is None:
        _pending_commands[name] = function
        return True
    return _current.add_command(
        name, function, documentation, single, local_name, encoded
    )


def remove_command(name: str) -> Any:
    _pending_commands.pop(name, None)
    if _current is None:
        return True
    return _current.remove_command(name)


def monitor_command(original: str, monitor: str) -> Any:
    if _current is None:
        _pending_monitors.append((original, monitor))
        return True
    return _current.monitor_command(original, monitor)


def remove_command_monitor(original: str, monitor: str) -> Any:
    global _pending_monitors
    if _current is None:
        _pending_monitors = [
            (o, m) for (o, m) in _pending_monitors if not (o == original and m == monitor)
        ]
        return True
    return _current.remove_command_monitor(original, monitor)


def reload(compile: bool = False) -> Any:
    if _current is None or _current_path is None:
        return _current
    return load_act_r_model(_current_path)


def _require():
    if _current is None:
        raise RuntimeError("no current model; call actr.load_act_r_model(...) first")
    return _current


def run(time: float, real_time: bool = False) -> Any:
    """Run like the tutorial client: returns ``[time, events, break]``."""
    return _require()._eval("run", time)


def run_full_time(time: float, real_time: bool = False) -> Any:
    return _require()._eval("run-full-time", time)


def run_until_time(time: float, real_time: bool = False) -> Any:
    return _require()._eval("run-until-time", time)


def run_n_events(count: int, real_time: bool = False) -> Any:
    return _require()._eval("run-n-events", count)


def __getattr__(name: str) -> Any:
    if name in {"connection", "current_connection"}:
        return None
    if name in {"stop", "connected_pipe_result"}:
        return lambda *args, **kwargs: None
    if name == "load_act_r_code":
        raise AttributeError("arbitrary Lisp source is not supported")
    if _current is None:
        raise AttributeError(
            "no current model; call actr.load_act_r_model(...) or actr.use(model) first"
        )
    return getattr(_current, name)


def __dir__() -> list[str]:
    return sorted(set(globals()) | (set(dir(_current)) if _current is not None else set()))
