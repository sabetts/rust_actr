"""Python port of ACT-R:tutorial;unit2;unit2-assignment-model.lisp."""

from __future__ import annotations

import rust_actr as actr

LOGICAL_PATH = 'ACT-R:tutorial;unit2;unit2-assignment-model.lisp'
MODEL_NAME = 'UNIT2'


def build(seed=(100, 0)) -> actr.Model:
    m = actr.Model('UNIT2', seed=seed)
    m.sgp(':V', True)
    m.sgp(':SHOW-FOCUS', True)
    m.chunk_type('READ-LETTERS', 'STEP')
    m.chunk_type('ARRAY', 'LETTER1', 'LETTER2', 'LETTER3')
    m.add_chunk('START', 'CHUNK', **{})
    m.goal_focus(isa='READ-LETTERS', **{'STEP': 'START'})
    return m
