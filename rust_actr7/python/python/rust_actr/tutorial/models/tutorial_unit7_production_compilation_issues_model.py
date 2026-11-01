"""Python port of ACT-R:tutorial;unit7;production-compilation-issues-model.lisp."""

from __future__ import annotations

import rust_actr as actr

LOGICAL_PATH = 'ACT-R:tutorial;unit7;production-compilation-issues-model.lisp'
MODEL_NAME = 'COMPILATION-TEST'


def build(seed=(100, 0)) -> actr.Model:
    m = actr.Model('COMPILATION-TEST', seed=seed)
    m.sgp(':ESC', True)
    m.sgp(':LF', 0.5)
    m.sgp(':BLL', 0.5)
    m.sgp(':MP', 18)
    m.sgp(':RT', -3)
    m.sgp(':ANS', 0.25)
    m.sgp(':STYLE-WARNINGS', None)
    m.sgp(':V', None)
    m.chunk_type('TASK', 'STATE')
    m.chunk_type('NUMBER', 'VISUAL-REP')
    m.chunk_type('RESPONSE', 'KEY', 'IS-RESPONSE', defaults={'IS_RESPONSE': True})
    m.chunk_type('TRIAL', 'NUM1', 'NUM2', 'RESULT', 'RESPONSE')
    m.define_chunks('WIN')
    m.define_chunks('DRAW')
    m.define_chunks('LOSE')
    m.define_chunks('ATTEND-NUM-1')
    m.define_chunks('ENCODE-NUM-1')
    m.define_chunks('FIND-NUM-2')
    m.define_chunks('ATTEND-NUM-2')
    m.define_chunks('ENCODE-NUM-2')
    m.define_chunks('NUM2')
    m.define_chunks('RETRIEVE-PAST-TRIAL')
    m.define_chunks('PROCESS-PAST-TRIAL')
    m.define_chunks('RESPOND')
    m.define_chunks('GUESS-OTHER')
    m.define_chunks('DETECT-FEEDBACK')
    m.define_chunks('ENCODE-FEEDBACK')
    m.add_chunk('ZERO', 'NUMBER', **{'VISUAL-REP': actr.String('0')})
    m.add_chunk('ONE', 'NUMBER', **{'VISUAL-REP': actr.String('1')})
    m.add_chunk('TWO', 'NUMBER', **{'VISUAL-REP': actr.String('2')})
    m.add_chunk('THREE', 'NUMBER', **{'VISUAL-REP': actr.String('3')})
    m.add_chunk('RESPONSE-1', 'RESPONSE', **{'KEY': actr.String('s')})
    m.add_chunk('RESPONSE-2', 'RESPONSE', **{'KEY': actr.String('d')})
    m.add_chunk('RESPONSE-3', 'RESPONSE', **{'KEY': actr.String('f')})
    m.set_similarities([('WIN', 'DRAW', -0.2), ('WIN', 'LOSE', -0.15), ('ZERO', 'ONE', -0.1), ('ONE', 'TWO', -0.1), ('TWO', 'THREE', -0.1), ('ZERO', 'TWO', -0.2), ('ONE', 'THREE', -0.2), ('ZERO', 'THREE', -0.3)])
    m.set_all_base_levels(1000, -100)
    m.goal_focus(isa='TASK', **{})
    m.add_production(
        '''(P DETECT-TRIAL-START =GOAL> ISA TASK STATE NIL =VISUAL-LOCATION> ISA VISUAL-LOCATION ?VISUAL> STATE FREE ?IMAGINAL> STATE FREE ==> =GOAL> STATE ATTEND-NUM-1 +IMAGINAL> ISA TRIAL +VISUAL> CMD MOVE-ATTENTION SCREEN-POS =VISUAL-LOCATION)'''
    )
    m.add_production(
        '''(P ATTEND-NUM-1 =GOAL> ISA TASK STATE ATTEND-NUM-1 =VISUAL> ISA VISUAL-OBJECT VALUE =VAL ==> =GOAL> STATE ENCODE-NUM-1 +RETRIEVAL> ISA NUMBER VISUAL-REP =VAL)'''
    )
    m.add_production(
        '''(P ENCODE-NUM-1 =GOAL> ISA TASK STATE ENCODE-NUM-1 =RETRIEVAL> ISA NUMBER =IMAGINAL> ISA TRIAL ==> =IMAGINAL> NUM1 =RETRIEVAL =GOAL> STATE FIND-NUM-2 +VISUAL-LOCATION> ISA VISUAL-LOCATION > SCREEN-X CURRENT :ATTENDED NIL)'''
    )
    m.add_production(
        '''(P FIND-NUM-2 =GOAL> ISA TASK STATE FIND-NUM-2 =VISUAL-LOCATION> ISA VISUAL-LOCATION ?VISUAL> STATE FREE ==> =GOAL> STATE ATTEND-NUM-2 +VISUAL> CMD MOVE-ATTENTION SCREEN-POS =VISUAL-LOCATION)'''
    )
    m.add_production(
        '''(P ATTEND-NUM-2 =GOAL> ISA TASK STATE ATTEND-NUM-2 =VISUAL> ISA VISUAL-OBJECT VALUE =VAL ==> =GOAL> STATE ENCODE-NUM-2 +RETRIEVAL> ISA NUMBER VISUAL-REP =VAL)'''
    )
    m.add_production(
        '''(P ENCODE-NUM-2 =GOAL> ISA TASK STATE ENCODE-NUM-2 =RETRIEVAL> ISA NUMBER =IMAGINAL> ISA TRIAL ==> =IMAGINAL> NUM2 =RETRIEVAL =GOAL> STATE RETRIEVE-PAST-TRIAL)'''
    )
    m.add_production(
        '''(P RETRIEVE-PAST-TRIAL =GOAL> ISA TASK STATE RETRIEVE-PAST-TRIAL =IMAGINAL> ISA TRIAL NUM1 =N1 NUM2 =N2 ==> =IMAGINAL> +RETRIEVAL> ISA TRIAL NUM1 =N1 NUM2 =N2 RESULT WIN =GOAL> STATE PROCESS-PAST-TRIAL)'''
    )
    m.add_production(
        '''(P NO-PAST-TRIAL =GOAL> ISA TASK STATE PROCESS-PAST-TRIAL ?RETRIEVAL> BUFFER FAILURE ==> +RETRIEVAL> ISA RESPONSE =GOAL> STATE RESPOND)'''
    )
    m.add_production(
        '''(P RETRIEVED-A-WIN =GOAL> ISA TASK STATE PROCESS-PAST-TRIAL =RETRIEVAL> ISA TRIAL RESULT WIN RESPONSE =RESPONSE ==> +RETRIEVAL> =RESPONSE =GOAL> STATE RESPOND)'''
    )
    m.add_production(
        '''(P RETRIEVED-A-NON-WIN =GOAL> ISA TASK STATE PROCESS-PAST-TRIAL =RETRIEVAL> ISA TRIAL - RESULT WIN RESPONSE =RESPONSE ==> +RETRIEVAL> =RESPONSE =GOAL> STATE GUESS-OTHER)'''
    )
    m.add_production(
        '''(P GUESS-OTHER =GOAL> ISA TASK STATE GUESS-OTHER =RETRIEVAL> ISA RESPONSE KEY =KEY ==> +RETRIEVAL> ISA RESPONSE - KEY =KEY =GOAL> STATE RESPOND)'''
    )
    m.add_production(
        '''(P RESPOND =GOAL> ISA TASK STATE RESPOND =RETRIEVAL> ISA RESPONSE KEY =KEY ?MANUAL> STATE FREE =IMAGINAL> ISA TRIAL ==> =IMAGINAL> RESPONSE =RETRIEVAL +MANUAL> CMD PRESS-KEY KEY =KEY =GOAL> STATE DETECT-FEEDBACK)'''
    )
    m.add_production(
        '''(P RESPOND-WHEN-RESPONSE-FAILURE =GOAL> ISA TASK STATE RESPOND ?RETRIEVAL> BUFFER FAILURE ?MANUAL> STATE FREE =IMAGINAL> ISA TRIAL ==> =IMAGINAL> RESPONSE RESPONSE-2 +MANUAL> CMD PRESS-KEY KEY "d" =GOAL> STATE DETECT-FEEDBACK)'''
    )
    m.add_production(
        '''(P DETECT-FEEDBACK =GOAL> ISA TASK STATE DETECT-FEEDBACK =VISUAL-LOCATION> ISA VISUAL-LOCATION ?VISUAL> STATE FREE ==> +VISUAL> CMD MOVE-ATTENTION SCREEN-POS =VISUAL-LOCATION =GOAL> STATE ENCODE-FEEDBACK)'''
    )
    m.add_production(
        '''(P ENCODE-FEEDBACK-WIN =GOAL> ISA TASK STATE ENCODE-FEEDBACK =IMAGINAL> ISA TRIAL =VISUAL> ISA VISUAL-OBJECT VALUE "win" ?MANUAL> STATE FREE ==> =IMAGINAL> RESULT WIN +MANUAL> CMD PRESS-KEY KEY SPACE +GOAL> ISA TASK)'''
    )
    m.add_production(
        '''(P ENCODE-FEEDBACK-LOSE =GOAL> ISA TASK STATE ENCODE-FEEDBACK =IMAGINAL> ISA TRIAL =VISUAL> ISA VISUAL-OBJECT VALUE "lose" ?MANUAL> STATE FREE ==> =IMAGINAL> RESULT LOSE +MANUAL> CMD PRESS-KEY KEY SPACE +GOAL> ISA TASK)'''
    )
    m.add_production(
        '''(P ENCODE-FEEDBACK-DRAW =GOAL> ISA TASK STATE ENCODE-FEEDBACK =IMAGINAL> ISA TRIAL =VISUAL> ISA VISUAL-OBJECT VALUE "draw" ?MANUAL> STATE FREE ==> =IMAGINAL> RESULT DRAW +MANUAL> CMD PRESS-KEY KEY SPACE +GOAL> ISA TASK)'''
    )
    return m
