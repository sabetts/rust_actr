"""Python interface to the Rust ACT-R engine.

Example::

    import rust_actr as actr

    m = actr.Model("count", seed=(100, 0))
    m.chunk_type("goal", "state")
    start = m.add_chunk("start", "goal", state="start")
    m.add_production('''
    (p finish
       =goal>
         state start
       ==>
       =goal>
         state done)
    ''')
    m.goal_focus(start)
    print(m.run(1.0))

Python strings are rendered as Lisp symbols. Wrap a value in
``actr.String(...)`` when the model needs a quoted Lisp string.
"""

from ._native import (
    BufferSpec,
    ChunkTypeSpec,
    ParamSpec,
    ResetPhase,
    __version__,
)
from ._model import Chunk, Model
from ._module import UNSET, Module
from ._values import LispString, String

__all__ = [
    "BufferSpec",
    "Chunk",
    "ChunkTypeSpec",
    "LispString",
    "Model",
    "Module",
    "ParamSpec",
    "ResetPhase",
    "String",
    "UNSET",
    "__version__",
]
