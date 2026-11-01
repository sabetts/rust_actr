"""Python port of ACT-R:tutorial;unit7;past-tense-model.lisp."""

from __future__ import annotations

import rust_actr as actr

LOGICAL_PATH = 'ACT-R:tutorial;unit7;past-tense-model.lisp'
MODEL_NAME = 'PAST-TENSE'


def build(seed=(100, 0)) -> actr.Model:
    m = actr.Model('PAST-TENSE', seed=seed)
    m.sgp(':V', True)
    m.sgp(':ESC', True)
    m.sgp(':UL', True)
    m.sgp(':OL', 6)
    m.sgp(':BLL', 0.5)
    m.sgp(':ANS', 0.1)
    m.sgp(':EGS', 0.2)
    m.sgp(':MAS', 3.5)
    m.sgp(':LF', 0.5)
    m.sgp(':RT', 0.5)
    m.sgp(':EPL', True)
    m.sgp(':ALPHA', 0.1)
    m.sgp(':PCT', True)
    m.sgp(':IU', 5)
    m.sgp(':NCNAR', None)
    m.sgp(':DO-NOT-HARVEST', 'IMAGINAL')
    m.chunk_type('PAST-TENSE', 'VERB', 'STEM', 'SUFFIX')
    m.chunk_type('GOAL', 'STATE')
    m.define_chunks('START')
    m.define_chunks('BLANK')
    m.define_chunks('DONE')
    m.declare_buffer_usage('GOAL', 'GOAL', 'STATE')
    m.declare_buffer_usage('IMAGINAL', 'PAST-TENSE', ':ALL')
    m.add_production(
        '''(P PAST-TENSE-IRREGULAR =GOAL> ISA GOAL STATE DONE =IMAGINAL> ISA PAST-TENSE VERB =WORD STEM =PAST SUFFIX BLANK ==> =GOAL> STATE NIL)'''
    )
    m.spp('PAST-TENSE-IRREGULAR', **{'REWARD': 5})
    m.add_production(
        '''(P PAST-TENSE-REGULAR =GOAL> ISA GOAL STATE DONE =IMAGINAL> ISA PAST-TENSE VERB =STEM STEM =STEM SUFFIX =SUFFIX !SAFE-EVAL! (NOT (EQ =SUFFIX (QUOTE BLANK))) ==> =GOAL> STATE NIL)'''
    )
    m.spp('PAST-TENSE-REGULAR', **{'REWARD': 4.2})
    m.add_production(
        '''(P NO-PAST-TENSE-FORMED =GOAL> ISA GOAL STATE DONE =IMAGINAL> ISA PAST-TENSE STEM NIL SUFFIX NIL ==> =GOAL> STATE NIL)'''
    )
    m.spp('NO-PAST-TENSE-FORMED', **{'REWARD': 3.9})
    return m
