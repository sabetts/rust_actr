"""Python-defined ACT-R modules.

Subclass :class:`Module` and override the hooks you need. The method set
mirrors Lisp's ``define-module``; hooks you do not override are no-ops.

The base class lives here (rather than in Rust) so the defaults use normal
Python values. ``ModuleBase`` is the PyO3 class that makes instances
recognizable to the native bridge.
"""

from __future__ import annotations

from typing import Any, Optional

from ._native import ModuleBase

#: Sentinel for ``Module.params``: its presence means "get the current value".
UNSET = object()


class Module(ModuleBase):
    """A module definition. Override only the hooks your module needs."""

    version = ""
    documentation = ""
    required: Any = ()
    requires: Any = ()

    def name(self) -> str:
        raise NotImplementedError("a module must provide name()")

    def buffers(self) -> list[Any]:
        return []

    def chunk_types(self) -> list[Any]:
        return []

    def parameters(self) -> list[Any]:
        return []

    def create(self, ctx: Any) -> None:
        pass

    def reset(self, ctx: Any, phase: Any) -> None:
        pass

    def request(self, ctx: Any, buffer: str, spec: Any) -> None:
        pass

    def buffer_mod(self, ctx: Any, buffer: str, spec: Any) -> None:
        pass

    def query(
        self, ctx: Any, buffer: str, name: str, value: Any
    ) -> Optional[bool]:
        return None

    def params(
        self, ctx: Any, name: str, value: Any = UNSET
    ) -> Optional[Any]:
        return None

    def delete(self, ctx: Any) -> None:
        pass

    def notify_on_clear(
        self, ctx: Any, buffer: str, chunk: Optional[str]
    ) -> None:
        pass

    def update(self, ctx: Any, old_time: float, new_time: float) -> None:
        pass

    def warning(self, ctx: Any, buffer: str, spec: Any) -> None:
        pass

    def search(self, ctx: Any, buffer: str) -> Optional[list[str]]:
        return None

    def offset(
        self, ctx: Any, buffer: str, chunks: list[str]
    ) -> Optional[list[float]]:
        return None

    def run_start(self, ctx: Any) -> None:
        pass

    def run_end(self, ctx: Any) -> None:
        pass

    def on_event(self, ctx: Any, event: Any) -> None:
        pass
