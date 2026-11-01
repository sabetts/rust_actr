"""Python port of ACT-R:tutorial;unit4;zbrodoff-model.lisp."""

from __future__ import annotations

import rust_actr as actr

LOGICAL_PATH = 'ACT-R:tutorial;unit4;zbrodoff-model.lisp'
MODEL_NAME = 'ZBRODOFF'


def build(seed=(100, 0)) -> actr.Model:
    m = actr.Model('ZBRODOFF', seed=seed)
    m.sgp(':V', None)
    m.sgp(':ESC', True)
    m.sgp(':LF', 0.4)
    m.sgp(':BLL', 0.5)
    m.sgp(':ANS', 0.5)
    m.sgp(':RT', 0)
    m.sgp(':NCNAR', None)
    m.sgp(':HEAR-SELF', None)
    m.sgp(':SHOW-FOCUS', True)
    m.chunk_type('PROBLEM', 'ARG1', 'ARG2', 'RESULT')
    m.chunk_type('GOAL', 'STATE', 'COUNT', 'TARGET', 'NEXT-LET', 'NEXT-NUM')
    m.chunk_type('NUMBER', 'NUMBER', 'NEXT', 'VISUAL-REP', 'VOCAL-REP')
    m.chunk_type('LETTER', 'LETTER', 'NEXT', 'VISUAL-REP', 'VOCAL-REP')
    m.add_chunk('ZERO', 'NUMBER', **{'NUMBER': 'ZERO', 'NEXT': 'ONE', 'VISUAL-REP': actr.String('0'), 'VOCAL-REP': actr.String('zero')})
    m.add_chunk('ONE', 'NUMBER', **{'NUMBER': 'ONE', 'NEXT': 'TWO', 'VISUAL-REP': actr.String('1'), 'VOCAL-REP': actr.String('one')})
    m.add_chunk('TWO', 'NUMBER', **{'NUMBER': 'TWO', 'NEXT': 'THREE', 'VISUAL-REP': actr.String('2'), 'VOCAL-REP': actr.String('two')})
    m.add_chunk('THREE', 'NUMBER', **{'NUMBER': 'THREE', 'NEXT': 'FOUR', 'VISUAL-REP': actr.String('3'), 'VOCAL-REP': actr.String('three')})
    m.add_chunk('FOUR', 'NUMBER', **{'NUMBER': 'FOUR', 'NEXT': 'FIVE', 'VISUAL-REP': actr.String('4'), 'VOCAL-REP': actr.String('four')})
    m.add_chunk('FIVE', 'NUMBER', **{'NUMBER': 'FIVE'})
    m.add_chunk('A', 'LETTER', **{'LETTER': 'A', 'NEXT': 'B', 'VISUAL-REP': actr.String('a'), 'VOCAL-REP': actr.String('a')})
    m.add_chunk('B', 'LETTER', **{'LETTER': 'B', 'NEXT': 'C', 'VISUAL-REP': actr.String('b'), 'VOCAL-REP': actr.String('b')})
    m.add_chunk('C', 'LETTER', **{'LETTER': 'C', 'NEXT': 'D', 'VISUAL-REP': actr.String('c'), 'VOCAL-REP': actr.String('c')})
    m.add_chunk('D', 'LETTER', **{'LETTER': 'D', 'NEXT': 'E', 'VISUAL-REP': actr.String('d'), 'VOCAL-REP': actr.String('d')})
    m.add_chunk('E', 'LETTER', **{'LETTER': 'E', 'NEXT': 'F', 'VISUAL-REP': actr.String('e'), 'VOCAL-REP': actr.String('e')})
    m.add_chunk('F', 'LETTER', **{'LETTER': 'F', 'NEXT': 'G', 'VISUAL-REP': actr.String('f'), 'VOCAL-REP': actr.String('f')})
    m.add_chunk('G', 'LETTER', **{'LETTER': 'G', 'NEXT': 'H', 'VISUAL-REP': actr.String('g'), 'VOCAL-REP': actr.String('g')})
    m.add_chunk('H', 'LETTER', **{'LETTER': 'H', 'NEXT': 'I', 'VISUAL-REP': actr.String('h'), 'VOCAL-REP': actr.String('h')})
    m.add_chunk('I', 'LETTER', **{'LETTER': 'I', 'NEXT': 'J', 'VISUAL-REP': actr.String('i'), 'VOCAL-REP': actr.String('i')})
    m.add_chunk('J', 'LETTER', **{'LETTER': 'J', 'NEXT': 'K', 'VISUAL-REP': actr.String('j'), 'VOCAL-REP': actr.String('j')})
    m.add_chunk('K', 'LETTER', **{'LETTER': 'K', 'NEXT': 'L', 'VISUAL-REP': actr.String('k'), 'VOCAL-REP': actr.String('k')})
    m.add_chunk('L', 'LETTER', **{'LETTER': 'L'})
    m.add_dm('ATTENDING')
    m.add_dm('READ')
    m.add_dm('COUNT')
    m.add_dm('COUNTING')
    m.add_dm('ENCODE')
    m.set_visloc_default(SCREEN_X='LOWEST')
    m.add_production(
        '''(P ATTEND =GOAL> ISA GOAL STATE NIL =VISUAL-LOCATION> ?VISUAL> STATE FREE ==> =GOAL> STATE ATTENDING +VISUAL> CMD MOVE-ATTENTION SCREEN-POS =VISUAL-LOCATION)'''
    )
    m.add_production(
        '''(P READ-FIRST =GOAL> ISA GOAL STATE ATTENDING =VISUAL> ISA VISUAL-OBJECT VALUE =CHAR ?IMAGINAL> BUFFER EMPTY STATE FREE ==> +IMAGINAL> +RETRIEVAL> ISA LETTER VISUAL-REP =CHAR =GOAL> STATE NIL +VISUAL-LOCATION> ISA VISUAL-LOCATION > SCREEN-X CURRENT SCREEN-X LOWEST - VALUE "+")'''
    )
    m.add_production(
        '''(P ENCODE-FIRST =GOAL> ISA GOAL STATE ATTENDING =RETRIEVAL> LETTER =LET VOCAL-REP =WORD =IMAGINAL> ARG1 NIL ?VOCAL> PREPARATION FREE ==> +VOCAL> CMD SUBVOCALIZE STRING =WORD =IMAGINAL> ARG1 =LET =GOAL> STATE READ)'''
    )
    m.add_production(
        '''(P READ-SECOND =GOAL> ISA GOAL STATE READ =VISUAL> ISA VISUAL-OBJECT VALUE =CHAR =IMAGINAL> ISA PROBLEM - ARG1 NIL ARG2 NIL ==> =IMAGINAL> +RETRIEVAL> ISA NUMBER VISUAL-REP =CHAR =GOAL> STATE NIL +VISUAL-LOCATION> ISA VISUAL-LOCATION SCREEN-X HIGHEST)'''
    )
    m.add_production(
        '''(P ENCODE-SECOND =GOAL> ISA GOAL STATE ATTENDING =RETRIEVAL> NUMBER =NUM VOCAL-REP =WORD =IMAGINAL> - ARG1 NIL ARG2 NIL ?VOCAL> PREPARATION FREE ==> +VOCAL> CMD SUBVOCALIZE STRING =WORD =IMAGINAL> ARG2 =NUM =GOAL> STATE READ)'''
    )
    m.add_production(
        '''(P READ-THIRD =GOAL> ISA GOAL STATE READ =IMAGINAL> ISA PROBLEM ARG1 =ARG1 ARG2 =ARG2 =VISUAL> ISA VISUAL-OBJECT VALUE =CHAR ?VISUAL> STATE FREE ==> =IMAGINAL> +RETRIEVAL> ISA LETTER VISUAL-REP =CHAR =GOAL> STATE ENCODE +VISUAL> CMD CLEAR)'''
    )
    m.add_production(
        '''(P ENCODE-THIRD =GOAL> ISA GOAL STATE ENCODE TARGET NIL =RETRIEVAL> LETTER =LET VOCAL-REP =WORD =IMAGINAL> ARG1 =A1 ARG2 =A2 ?VOCAL> PREPARATION FREE ==> +VOCAL> CMD SUBVOCALIZE STRING =WORD =GOAL> TARGET =LET STATE COUNT =IMAGINAL>)'''
    )
    m.add_production(
        '''(P START-COUNTING =GOAL> ISA GOAL STATE COUNT =IMAGINAL> ISA PROBLEM ARG1 =A ARG2 =VAL ==> =IMAGINAL> RESULT =A =GOAL> STATE COUNTING +RETRIEVAL> ISA LETTER LETTER =A)'''
    )
    m.add_production(
        '''(P INITIALIZE-COUNTING =GOAL> ISA GOAL STATE COUNTING COUNT NIL =RETRIEVAL> ISA LETTER LETTER =L NEXT =NEW VOCAL-REP =T ?VOCAL> STATE FREE ==> +VOCAL> ISA SUBVOCALIZE STRING =T =GOAL> NEXT-NUM ONE NEXT-LET =NEW COUNT ZERO +RETRIEVAL> ISA LETTER LETTER =NEW)'''
    )
    m.add_production(
        '''(P UPDATE-RESULT =GOAL> ISA GOAL COUNT =VAL NEXT-LET =LET NEXT-NUM =N =IMAGINAL> ISA PROBLEM - ARG2 =VAL =RETRIEVAL> ISA LETTER LETTER =LET NEXT =NEW VOCAL-REP =TXT ?VOCAL> STATE FREE ==> =GOAL> NEXT-LET =NEW +VOCAL> CMD SUBVOCALIZE STRING =TXT =IMAGINAL> RESULT =LET +RETRIEVAL> ISA NUMBER NUMBER =N)'''
    )
    m.add_production(
        '''(P UPDATE-COUNT =GOAL> ISA GOAL NEXT-LET =LET NEXT-NUM =N =RETRIEVAL> ISA NUMBER NUMBER =VAL NEXT =NEW VOCAL-REP =TXT ?VOCAL> PREPARATION FREE ==> +VOCAL> CMD SUBVOCALIZE STRING =TXT =GOAL> COUNT =VAL NEXT-NUM =NEW +RETRIEVAL> ISA LETTER LETTER =LET)'''
    )
    m.add_production(
        '''(P FINAL-ANSWER-YES =GOAL> ISA GOAL TARGET =LET COUNT =VAL =IMAGINAL> ISA PROBLEM RESULT =LET ARG2 =VAL ?MANUAL> STATE FREE ==> +GOAL> +MANUAL> CMD PRESS-KEY KEY "k")'''
    )
    m.add_production(
        '''(P FINAL-ANSWER-NO =GOAL> ISA GOAL COUNT =VAL - TARGET =LET =IMAGINAL> ISA PROBLEM RESULT =LET ARG2 =VAL ?MANUAL> STATE FREE ==> +GOAL> +MANUAL> CMD PRESS-KEY KEY "d")'''
    )
    m.set_all_base_levels(100000, -1000)
    m.goal_focus(isa='GOAL', **{})
    return m
