"""Python port of ACT-R:tutorial;unit7;utility-learning-issues-model.lisp."""

from __future__ import annotations

import rust_actr as actr

LOGICAL_PATH = 'ACT-R:tutorial;unit7;utility-learning-issues-model.lisp'
MODEL_NAME = 'UTILITY-LEARNING-TEST'


def build(seed=(100, 0)) -> actr.Model:
    m = actr.Model('UTILITY-LEARNING-TEST', seed=seed)
    m.sgp(':SEED', [100, 2])
    m.sgp(':ESC', True)
    m.sgp(':UL', True)
    m.sgp(':EGS', 0.5)
    m.chunk_type('CHOOSE', 'CHOICE')
    m.chunk_type('RESPONSE', 'ANSWER')
    m.define_chunks('A')
    m.define_chunks('B')
    m.declare_buffer_usage('IMAGINAL', 'RESPONSE', 'ANSWER')
    m.add_production(
        '''(P CHOOSE-A =GOAL> ISA CHOOSE CHOICE NIL ==> =GOAL> CHOICE A)'''
    )
    m.add_production(
        '''(P CHOOSE-B =GOAL> ISA CHOOSE CHOICE NIL ==> =GOAL> CHOICE B)'''
    )
    m.add_production(
        '''(P RESPONSE-MATCHES =GOAL> ISA CHOOSE CHOICE =CHOICE =IMAGINAL> ISA RESPONSE ANSWER =CHOICE ==> -GOAL>)'''
    )
    m.add_production(
        '''(P RESPONSE-DOESNT-MATCH =GOAL> ISA CHOOSE CHOICE =CHOICE =IMAGINAL> ISA RESPONSE - ANSWER =CHOICE - ANSWER NIL ==> -GOAL>)'''
    )
    m.add_production(
        '''(P UNKNOWN-RESPONSE =GOAL> ISA CHOOSE CHOICE =CHOICE =IMAGINAL> ISA RESPONSE ANSWER NIL ==> -GOAL>)'''
    )
    m.spp('RESPONSE-MATCHES', **{'REWARD': 4})
    m.spp('RESPONSE-DOESNT-MATCH', **{'REWARD': 0})
    m.form('(SCHEDULE-EVENT 0 "utility-learning-issues-choose" :OUTPUT (QUOTE MEDIUM))')
    return m
