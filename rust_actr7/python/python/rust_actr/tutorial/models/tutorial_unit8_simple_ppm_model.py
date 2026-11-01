"""Python port of ACT-R:tutorial;unit8;simple-ppm-model.lisp."""

from __future__ import annotations

import rust_actr as actr

LOGICAL_PATH = 'ACT-R:tutorial;unit8;simple-ppm-model.lisp'
MODEL_NAME = 'TEST-PPM'


def build(seed=(100, 0)) -> actr.Model:
    m = actr.Model('TEST-PPM', seed=seed)
    m.sgp(':V', True)
    m.sgp(':TRACE-DETAIL', 'HIGH')
    m.sgp(':ESC', True)
    m.sgp(':PPM', None)
    m.sgp(':EGS', 0)
    m.sgp(':CST', True)
    m.add_chunk('SMALL', 'CHUNK', **{})
    m.add_chunk('MEDIUM', 'CHUNK', **{})
    m.add_chunk('LARGE', 'CHUNK', **{})
    m.chunk_type('TEST', 'VALUE')
    m.set_similarities([('SMALL', 'MEDIUM', -0.5), ('MEDIUM', 'LARGE', -0.5)])
    m.goal_focus(isa='TEST', **{'VALUE': 'SMALL'})
    m.add_production(
        '''(P SMALL =GOAL> ISA TEST VALUE SMALL VALUE =VAL ==> !OUTPUT! =VAL -GOAL>)'''
    )
    m.add_production(
        '''(P MEDIUM =GOAL> ISA TEST VALUE MEDIUM VALUE =VAL ==> !OUTPUT! =VAL -GOAL>)'''
    )
    m.add_production(
        '''(P LARGE =GOAL> ISA TEST VALUE LARGE VALUE =VAL ==> !OUTPUT! =VAL -GOAL>)'''
    )
    return m
