"""Python port of ACT-R:tutorial;unit3;perceptual-motor-issues-model.lisp."""

from __future__ import annotations

import rust_actr as actr

LOGICAL_PATH = 'ACT-R:tutorial;unit3;perceptual-motor-issues-model.lisp'
MODEL_NAME = 'PERCEPTUAL-MOTOR-ISSUES'


def build(seed=(100, 0)) -> actr.Model:
    m = actr.Model('PERCEPTUAL-MOTOR-ISSUES', seed=seed)
    m.sgp(':SEED', [101, 0])
    m.sgp(':V', True)
    m.sgp(':SHOW-FOCUS', True)
    m.sgp(':TRACE-DETAIL', 'MEDIUM')
    m.sgp(':ER', True)
    m.chunk_type('LETTER', 'NAME', 'NEXT', 'PREVIOUS')
    m.chunk_type('TASK', 'LETTER')
    m.add_chunk('A', 'LETTER', **{'NAME': actr.String('a'), 'NEXT': actr.String('b'), 'PREVIOUS': actr.String('z')})
    m.add_chunk('B', 'LETTER', **{'NAME': actr.String('b'), 'NEXT': actr.String('c'), 'PREVIOUS': actr.String('a')})
    m.add_chunk('C', 'LETTER', **{'NAME': actr.String('c'), 'NEXT': actr.String('d'), 'PREVIOUS': actr.String('b')})
    m.add_chunk('D', 'LETTER', **{'NAME': actr.String('d'), 'NEXT': actr.String('e'), 'PREVIOUS': actr.String('c')})
    m.add_chunk('E', 'LETTER', **{'NAME': actr.String('e'), 'NEXT': actr.String('f'), 'PREVIOUS': actr.String('d')})
    m.add_chunk('F', 'LETTER', **{'NAME': actr.String('f'), 'NEXT': actr.String('g'), 'PREVIOUS': actr.String('e')})
    m.add_chunk('G', 'LETTER', **{'NAME': actr.String('g'), 'NEXT': actr.String('h'), 'PREVIOUS': actr.String('f')})
    m.add_chunk('H', 'LETTER', **{'NAME': actr.String('h'), 'NEXT': actr.String('i'), 'PREVIOUS': actr.String('g')})
    m.add_chunk('I', 'LETTER', **{'NAME': actr.String('i'), 'NEXT': actr.String('j'), 'PREVIOUS': actr.String('h')})
    m.add_chunk('J', 'LETTER', **{'NAME': actr.String('j'), 'NEXT': actr.String('k'), 'PREVIOUS': actr.String('i')})
    m.add_chunk('K', 'LETTER', **{'NAME': actr.String('k'), 'NEXT': actr.String('l'), 'PREVIOUS': actr.String('j')})
    m.add_chunk('L', 'LETTER', **{'NAME': actr.String('l'), 'NEXT': actr.String('m'), 'PREVIOUS': actr.String('k')})
    m.add_chunk('M', 'LETTER', **{'NAME': actr.String('m'), 'NEXT': actr.String('n'), 'PREVIOUS': actr.String('l')})
    m.add_chunk('N', 'LETTER', **{'NAME': actr.String('n'), 'NEXT': actr.String('o'), 'PREVIOUS': actr.String('m')})
    m.add_chunk('O', 'LETTER', **{'NAME': actr.String('o'), 'NEXT': actr.String('p'), 'PREVIOUS': actr.String('n')})
    m.add_chunk('P', 'LETTER', **{'NAME': actr.String('p'), 'NEXT': actr.String('q'), 'PREVIOUS': actr.String('o')})
    m.add_chunk('Q', 'LETTER', **{'NAME': actr.String('q'), 'NEXT': actr.String('r'), 'PREVIOUS': actr.String('p')})
    m.add_chunk('R', 'LETTER', **{'NAME': actr.String('r'), 'NEXT': actr.String('s'), 'PREVIOUS': actr.String('q')})
    m.add_chunk('S', 'LETTER', **{'NAME': actr.String('s'), 'NEXT': actr.String('t'), 'PREVIOUS': actr.String('r')})
    m.add_chunk('T', 'LETTER', **{'NAME': actr.String('t'), 'NEXT': actr.String('u'), 'PREVIOUS': actr.String('s')})
    m.add_chunk('U', 'LETTER', **{'NAME': actr.String('u'), 'NEXT': actr.String('v'), 'PREVIOUS': actr.String('t')})
    m.add_chunk('V', 'LETTER', **{'NAME': actr.String('v'), 'NEXT': actr.String('w'), 'PREVIOUS': actr.String('u')})
    m.add_chunk('W', 'LETTER', **{'NAME': actr.String('w'), 'NEXT': actr.String('x'), 'PREVIOUS': actr.String('v')})
    m.add_chunk('X', 'LETTER', **{'NAME': actr.String('x'), 'NEXT': actr.String('y'), 'PREVIOUS': actr.String('w')})
    m.add_chunk('Y', 'LETTER', **{'NAME': actr.String('y'), 'NEXT': actr.String('z'), 'PREVIOUS': actr.String('x')})
    m.add_chunk('Z', 'LETTER', **{'NAME': actr.String('z'), 'NEXT': actr.String('a'), 'PREVIOUS': actr.String('y')})
    m.add_chunk('START', 'CHUNK', **{})
    m.add_chunk('ATTEND', 'CHUNK', **{})
    m.add_chunk('RESPOND', 'CHUNK', **{})
    m.add_chunk('DONE', 'CHUNK', **{})
    m.add_production(
        '''(P FIND-LETTER =VISUAL-LOCATION> ?VISUAL> STATE FREE ==> +VISUAL> CMD MOVE-ATTENTION SCREEN-POS =VISUAL-LOCATION +IMAGINAL>)'''
    )
    m.add_production(
        '''(P ENCODE-LETTER =IMAGINAL> ISA TASK LETTER NIL =VISUAL> ISA VISUAL-OBJECT VALUE =LETTER ==> =IMAGINAL> LETTER =LETTER)'''
    )
    m.add_production(
        '''(P RESPOND-NEXT =IMAGINAL> ISA TASK LETTER =LETTER =VISUAL> ISA VISUAL-OBJECT VALUE "next" ?MANUAL> STATE BUSY ==> +RETRIEVAL> ISA LETTER PREVIOUS =LETTER +MANUAL> CMD PRESS-KEY KEY =LETTER)'''
    )
    m.add_production(
        '''(P RESPOND-PREVIOUS =IMAGINAL> ISA TASK LETTER =LETTER =VISUAL> ISA TEXT VALUE "previous" ?MANUAL> STATE FREE ==> +RETRIEVAL> ISA LETTER NEXT =LETTER +MANUAL> CMD PRESS-KEY KEY =LETTER)'''
    )
    m.add_production(
        '''(P RESPOND-FINAL =RETRIEVAL> ISA LETTER NAME =LETTER ==> +MANUAL> CMD PRESS-KEY KEY =LETTER)'''
    )
    return m
