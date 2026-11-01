"""Python port of ACT-R:tutorial;unit6;choice-model.lisp."""

from __future__ import annotations

import rust_actr as actr

LOGICAL_PATH = 'ACT-R:tutorial;unit6;choice-model.lisp'
MODEL_NAME = 'CHOICE'


def build(seed=(100, 0)) -> actr.Model:
    m = actr.Model('CHOICE', seed=seed)
    return m
