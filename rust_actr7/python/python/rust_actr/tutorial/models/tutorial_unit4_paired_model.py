"""Python port of ACT-R:tutorial;unit4;paired-model.lisp."""

from __future__ import annotations

import rust_actr as actr

LOGICAL_PATH = 'ACT-R:tutorial;unit4;paired-model.lisp'
MODEL_NAME = 'PAIRED'


def build(seed=(100, 0)) -> actr.Model:
    m = actr.Model('PAIRED', seed=seed)
    m.sgp(':V', True)
    m.sgp(':ESC', True)
    m.sgp(':RT', -2)
    m.sgp(':LF', 0.4)
    m.sgp(':ANS', 0.5)
    m.sgp(':BLL', 0.5)
    m.sgp(':ACT', None)
    m.sgp(':NCNAR', None)
    m.sgp(':SEED', [200, 4])
    m.chunk_type('GOAL', 'STEP')
    m.chunk_type('PAIR', 'PROBE', 'ANSWER')
    m.add_chunk('START', 'CHUNK', **{})
    m.add_chunk('ATTENDING-TARGET', 'CHUNK', **{})
    m.add_chunk('ATTENDING-PROBE', 'CHUNK', **{})
    m.add_chunk('TESTING', 'CHUNK', **{})
    m.add_chunk('READ-STUDY-ITEM', 'CHUNK', **{})
    m.add_production(
        '''(P ATTEND-PROBE =GOAL> ISA GOAL STEP START =VISUAL-LOCATION> ?VISUAL> STATE FREE ==> +VISUAL> CMD MOVE-ATTENTION SCREEN-POS =VISUAL-LOCATION =GOAL> STEP ATTENDING-PROBE)'''
    )
    m.add_production(
        '''(P READ-PROBE =GOAL> ISA GOAL STEP ATTENDING-PROBE =VISUAL> ISA VISUAL-OBJECT VALUE =VAL ?IMAGINAL> STATE FREE ==> +IMAGINAL> ISA PAIR PROBE =VAL +RETRIEVAL> ISA PAIR PROBE =VAL =GOAL> STEP TESTING)'''
    )
    m.add_production(
        '''(P RECALL =GOAL> ISA GOAL STEP TESTING =RETRIEVAL> ISA PAIR ANSWER =ANS ?MANUAL> STATE FREE ?VISUAL> STATE FREE ==> +MANUAL> CMD PRESS-KEY KEY =ANS =GOAL> STEP READ-STUDY-ITEM +VISUAL> CMD CLEAR)'''
    )
    m.add_production(
        '''(P CANNOT-RECALL =GOAL> ISA GOAL STEP TESTING ?RETRIEVAL> BUFFER FAILURE ?VISUAL> STATE FREE ==> =GOAL> STEP READ-STUDY-ITEM +VISUAL> CMD CLEAR)'''
    )
    m.add_production(
        '''(P DETECT-STUDY-ITEM =GOAL> ISA GOAL STEP READ-STUDY-ITEM =VISUAL-LOCATION> ?VISUAL> STATE FREE ==> +VISUAL> CMD MOVE-ATTENTION SCREEN-POS =VISUAL-LOCATION =GOAL> STEP ATTENDING-TARGET)'''
    )
    m.add_production(
        '''(P ASSOCIATE =GOAL> ISA GOAL STEP ATTENDING-TARGET =VISUAL> ISA VISUAL-OBJECT VALUE =VAL =IMAGINAL> ISA PAIR PROBE =PROBE ?VISUAL> STATE FREE ==> =IMAGINAL> ANSWER =VAL -IMAGINAL> =GOAL> STEP START +VISUAL> CMD CLEAR)'''
    )
    m.goal_focus(isa='GOAL', **{'STEP': 'START'})
    return m
