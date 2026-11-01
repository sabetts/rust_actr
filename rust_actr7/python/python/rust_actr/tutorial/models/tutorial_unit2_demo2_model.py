"""Python port of ACT-R:tutorial;unit2;demo2-model.lisp."""

from __future__ import annotations

import rust_actr as actr

LOGICAL_PATH = 'ACT-R:tutorial;unit2;demo2-model.lisp'
MODEL_NAME = 'DEMO2'


def build(seed=(100, 0)) -> actr.Model:
    m = actr.Model('DEMO2', seed=seed)
    m.sgp(':SEED', [123456, 0])
    m.sgp(':V', True)
    m.sgp(':SHOW-FOCUS', True)
    m.sgp(':TRACE-DETAIL', 'HIGH')
    m.chunk_type('READ-LETTERS', 'STEP')
    m.chunk_type('ARRAY', 'LETTER')
    m.add_chunk('START', 'CHUNK', **{})
    m.add_chunk('ATTEND', 'CHUNK', **{})
    m.add_chunk('RESPOND', 'CHUNK', **{})
    m.add_chunk('DONE', 'CHUNK', **{})
    m.add_production(
        '''(P FIND-UNATTENDED-LETTER =GOAL> ISA READ-LETTERS STEP START ==> +VISUAL-LOCATION> :ATTENDED NIL =GOAL> STEP FIND-LOCATION)'''
    )
    m.add_production(
        '''(P ATTEND-LETTER =GOAL> ISA READ-LETTERS STEP FIND-LOCATION =VISUAL-LOCATION> ?VISUAL> STATE FREE ==> +VISUAL> CMD MOVE-ATTENTION SCREEN-POS =VISUAL-LOCATION =GOAL> STEP ATTEND)'''
    )
    m.add_production(
        '''(P ENCODE-LETTER =GOAL> ISA READ-LETTERS STEP ATTEND =VISUAL> VALUE =LETTER ?IMAGINAL> STATE FREE ==> =GOAL> STEP RESPOND +IMAGINAL> ISA ARRAY LETTER =LETTER)'''
    )
    m.add_production(
        '''(P RESPOND =GOAL> ISA READ-LETTERS STEP RESPOND =IMAGINAL> ISA ARRAY LETTER =LETTER ?MANUAL> STATE FREE ==> =GOAL> STEP DONE +MANUAL> CMD PRESS-KEY KEY =LETTER)'''
    )
    m.goal_focus(isa='READ-LETTERS', **{'STEP': 'START'})
    return m
