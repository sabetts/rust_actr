"""Python port of ACT-R:tutorial;unit1;count.lisp."""

from __future__ import annotations

import rust_actr as actr

LOGICAL_PATH = 'ACT-R:tutorial;unit1;count.lisp'
MODEL_NAME = 'COUNT'


def build(seed=(100, 0)) -> actr.Model:
    m = actr.Model('COUNT', seed=seed)
    m.sgp(':ESC', True)
    m.sgp(':LF', 0.05)
    m.sgp(':TRACE-DETAIL', 'HIGH')
    m.chunk_type('NUMBER', 'NUMBER', 'NEXT')
    m.chunk_type('COUNT-FROM', 'START', 'END', 'COUNT')
    m.add_chunk('ONE', 'NUMBER', **{'NUMBER': 'ONE', 'NEXT': 'TWO'})
    m.add_chunk('TWO', 'NUMBER', **{'NUMBER': 'TWO', 'NEXT': 'THREE'})
    m.add_chunk('THREE', 'NUMBER', **{'NUMBER': 'THREE', 'NEXT': 'FOUR'})
    m.add_chunk('FOUR', 'NUMBER', **{'NUMBER': 'FOUR', 'NEXT': 'FIVE'})
    m.add_chunk('FIVE', 'NUMBER', **{'NUMBER': 'FIVE'})
    m.goal_focus(isa='COUNT-FROM', **{'START': 'TWO', 'END': 'FOUR'})
    m.add_production(
        '''(P START =GOAL> ISA COUNT-FROM START =NUM1 COUNT NIL ==> =GOAL> ISA COUNT-FROM COUNT =NUM1 +RETRIEVAL> ISA NUMBER NUMBER =NUM1)'''
    )
    m.add_production(
        '''(P INCREMENT =GOAL> ISA COUNT-FROM COUNT =NUM1 - END =NUM1 =RETRIEVAL> ISA NUMBER NUMBER =NUM1 NEXT =NUM2 ==> =GOAL> ISA COUNT-FROM COUNT =NUM2 +RETRIEVAL> ISA NUMBER NUMBER =NUM2 !OUTPUT! (=NUM1))'''
    )
    m.add_production(
        '''(P STOP =GOAL> ISA COUNT-FROM COUNT =NUM END =NUM =RETRIEVAL> ISA NUMBER NUMBER =NUM ==> -GOAL> !OUTPUT! (=NUM))'''
    )
    return m
