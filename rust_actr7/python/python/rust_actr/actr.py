"""A module-level façade matching the tutorial ``actr`` client.

The self-contained tutorial scripts call functions such as ``actr.run`` and
``actr.add_command``.  This module binds those calls to the current
:class:`rust_actr.Model`.  Build a model and make it current with ``use()``::

    import rust_actr.actr as actr

    actr.use(actr.Model("count"))
    print(actr.run(10))
"""

from __future__ import annotations

from typing import Any, Optional

from ._model import Chunk, Model
from ._values import LispString, String

_current: Optional[Any] = None
_pending_commands: dict[str, Any] = {}
_pending_monitors: list[tuple[str, str]] = []


def clear_state() -> None:
    """Drop the current model and any commands queued before it was bound."""
    global _current, _pending_commands, _pending_monitors
    _current = None
    _pending_commands = {}
    _pending_monitors = []


def use(model: Any) -> Any:
    """Bind ``model`` as the current model and replay pending commands."""
    global _current
    _current = model
    for name, function in _pending_commands.items():
        _current.add_command(name, function)
    for original, monitor in _pending_monitors:
        _current.monitor_command(original, monitor)
    hook = getattr(_current, "sim_hook_name", None)
    hook_fn = _pending_commands.get(hook.upper()) if hook else None
    if hook_fn is not None:
        _current.set_sim_hook(hook_fn)
    return model


def current_model() -> Optional[Any]:
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
        _pending_commands[name.upper()] = function
        return True
    result = _current.add_command(
        name, function, documentation, single, local_name, encoded
    )
    hook = getattr(_current, "sim_hook_name", None)
    if function is not None and hook and name.upper() == hook.upper():
        _current.set_sim_hook(function)
    return result


def remove_command(name: str) -> Any:
    _pending_commands.pop(name.upper(), None)
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


def _require():
    if _current is None:
        raise RuntimeError("no current model; call actr.use(model) first")
    return _current


def run(time: float, real_time: bool = False) -> Any:
    """Run like the tutorial client: returns ``[time, events, break]``."""
    return _require()._run_result("run", time)


def run_full_time(time: float, real_time: bool = False) -> Any:
    return _require()._run_result("run-full-time", time)


def run_until_time(time: float, real_time: bool = False) -> Any:
    return _require()._run_result("run-until-time", time)


def run_n_events(count: int, real_time: bool = False) -> Any:
    return _require()._run_result("run-n-events", count)


def __getattr__(name: str) -> Any:
    if name in {"connection", "current_connection"}:
        return None
    if name in {"stop", "connected_pipe_result"}:
        return lambda *args, **kwargs: None
    if name == "load_act_r_code":
        raise AttributeError("arbitrary Lisp source is not supported")
    if name == "load_act_r_model":
        raise AttributeError(
            "load_act_r_model is not supported; import a tutorial module "
            "and call actr.use(module.build())"
        )
    if _current is None:
        raise AttributeError(
            "no current model; call actr.use(model) first"
        )
    return getattr(_current, name)


def __dir__() -> list[str]:
    return sorted(set(globals()) | (set(dir(_current)) if _current is not None else set()))
