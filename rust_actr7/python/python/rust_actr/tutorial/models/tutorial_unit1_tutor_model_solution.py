"""Python port of ACT-R:tutorial;unit1;tutor-model-solution.lisp."""

from __future__ import annotations

import rust_actr as actr

LOGICAL_PATH = 'ACT-R:tutorial;unit1;tutor-model-solution.lisp'
MODEL_NAME = 'TUTOR-MODEL'


def build(seed=(100, 0)) -> actr.Model:
    m = actr.Model('TUTOR-MODEL', seed=seed)
    m.sgp(':ESC', True)
    m.sgp(':LF', 0.05)
    m.sgp(':TRACE-DETAIL', 'MEDIUM')
    m.chunk_type('ADDITION-FACT', 'ADDEND1', 'ADDEND2', 'SUM')
    m.chunk_type('ADD-PAIR', 'ONE1', 'TEN1', 'ONE2', 'TEN2', 'TEN-ANS', 'ONE-ANS', 'CARRY')
    m.add_chunk('FACT17', 'ADDITION-FACT', **{'ADDEND1': 1, 'ADDEND2': 7, 'SUM': 8})
    m.add_chunk('FACT34', 'ADDITION-FACT', **{'ADDEND1': 3, 'ADDEND2': 4, 'SUM': 7})
    m.add_chunk('FACT67', 'ADDITION-FACT', **{'ADDEND1': 6, 'ADDEND2': 7, 'SUM': 13})
    m.add_chunk('FACT103', 'ADDITION-FACT', **{'ADDEND1': 10, 'ADDEND2': 3, 'SUM': 13})
    m.add_production(
        '''(P START-PAIR =GOAL> ISA ADD-PAIR ONE1 =NUM1 ONE2 =NUM2 ONE-ANS NIL ==> =GOAL> ONE-ANS BUSY +RETRIEVAL> ISA ADDITION-FACT ADDEND1 =NUM1 ADDEND2 =NUM2)'''
    )
    m.add_production(
        '''(P ADD-ONES =GOAL> ISA ADD-PAIR ONE-ANS BUSY ONE1 =NUM1 ONE2 =NUM2 =RETRIEVAL> ISA ADDITION-FACT ADDEND1 =NUM1 ADDEND2 =NUM2 SUM =SUM ==> =GOAL> ONE-ANS =SUM CARRY BUSY +RETRIEVAL> ISA ADDITION-FACT ADDEND1 10 SUM =SUM)'''
    )
    m.add_production(
        '''(P PROCESS-CARRY =GOAL> ISA ADD-PAIR TEN1 =NUM1 TEN2 =NUM2 CARRY BUSY ONE-ANS =ONES =RETRIEVAL> ISA ADDITION-FACT ADDEND1 10 SUM =ONES ADDEND2 =REMAINDER ==> =GOAL> CARRY 1 TEN-ANS BUSY ONE-ANS =REMAINDER +RETRIEVAL> ISA ADDITION-FACT ADDEND1 =NUM1 ADDEND2 =NUM2)'''
    )
    m.add_production(
        '''(P NO-CARRY =GOAL> ISA ADD-PAIR TEN1 =NUM1 TEN2 =NUM2 CARRY BUSY ?RETRIEVAL> BUFFER FAILURE ==> =GOAL> CARRY NIL TEN-ANS BUSY +RETRIEVAL> ISA ADDITION-FACT ADDEND1 =NUM1 ADDEND2 =NUM2)'''
    )
    m.add_production(
        '''(P ADD-TENS-DONE =GOAL> ISA ADD-PAIR TEN-ANS BUSY CARRY NIL =RETRIEVAL> ISA ADDITION-FACT SUM =SUM ==> =GOAL> TEN-ANS =SUM)'''
    )
    m.add_production(
        '''(P ADD-TENS-CARRY =GOAL> ISA ADD-PAIR CARRY 1 TEN-ANS BUSY =RETRIEVAL> ISA ADDITION-FACT SUM =SUM ==> =GOAL> CARRY NIL +RETRIEVAL> ISA ADDITION-FACT ADDEND1 1 ADDEND2 =SUM)'''
    )
    m.goal_focus(isa='ADD-PAIR', **{'TEN1': 3, 'ONE1': 6, 'TEN2': 4, 'ONE2': 7})
    return m
