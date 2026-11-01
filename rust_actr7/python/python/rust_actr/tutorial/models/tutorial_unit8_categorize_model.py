"""Python port of ACT-R:tutorial;unit8;categorize-model.lisp."""

from __future__ import annotations

import rust_actr as actr

LOGICAL_PATH = 'ACT-R:tutorial;unit8;categorize-model.lisp'
MODEL_NAME = 'CATEGORIZE'


def build(seed=(100, 0)) -> actr.Model:
    m = actr.Model('CATEGORIZE', seed=seed)
    m.sgp(':V', True)
    m.sgp(':ACT', True)
    m.sgp(':ESC', True)
    m.sgp(':LF', 0.01)
    m.sgp(':BLC', 10)
    m.sgp(':RT', -100)
    m.sgp(':DO-NOT-HARVEST', 'IMAGINAL')
    m.sgp(':SIM-HOOK', actr.String('size-similarities'))
    m.sgp(':CACHE-SIM-HOOK-RESULTS', True)
    m.sgp(':MP', 1)
    m.sgp(':PPM', 1)
    m.sgp(':EGS', 0.25)
    m.sgp(':ANS', 0.25)
    m.chunk_type('GOAL', 'STATE', 'NAME', 'VALUE')
    m.chunk_type('EXAMPLE', 'CATEGORY')
    m.add_chunk('SMALL', 'CHUNK', **{})
    m.add_chunk('MEDIUM', 'CHUNK', **{})
    m.add_chunk('LARGE', 'CHUNK', **{})
    m.define_chunks('UNKNOWN')
    m.define_chunks('CATEGORIZE')
    m.define_chunks('ADD-ATTRIBUTE')
    m.call_act_r_command('create-example-memories')
    m.set_similarities([('SMALL', 'MEDIUM', -0.4), ('MEDIUM', 'LARGE', -0.4), ('SMALL', 'LARGE', -0.8)])
    m.form('(SET-BUFFER-CHUNK (QUOTE IMAGINAL) (QUOTE (CATEGORY UNKNOWN)))')
    m.declare_buffer_usage('GOAL', 'GOAL', ':ALL')
    return m
