"""Python port of ACT-R:tutorial;unit5;declarative-issues.lisp."""

from __future__ import annotations

import rust_actr as actr

LOGICAL_PATH = 'ACT-R:tutorial;unit5;declarative-issues.lisp'
MODEL_NAME = 'DECLARATIVE-ISSUES'


def build(seed=(100, 0)) -> actr.Model:
    m = actr.Model('DECLARATIVE-ISSUES', seed=seed)
    m.sgp(':SEED', [100, 1])
    m.sgp(':ESC', True)
    m.sgp(':V', True)
    m.sgp(':BLL', 0.5)
    m.sgp(':ANS', 0.25)
    m.sgp(':MAS', 2)
    m.sgp(':MP', 10)
    m.chunk_type('SIMPLE-VALUE', 'RESULT')
    m.chunk_type('SIMPLE-FACT', 'ITEM', 'ATTRIBUTE')
    m.chunk_type('TASK-STATE', 'STATE')
    m.chunk_type('NUMBER', 'VISUAL-REP')
    m.chunk_type('MATH-FACT', 'ARG1', 'ARG2', 'RESULT', 'OPERATOR')
    m.chunk_type('CONTEXT', 'VAL1', 'VAL2', 'VAL3', 'GOAL')
    m.define_chunks(actr.Chunk('R2', 'CHUNK', {}))
    m.define_chunks(actr.Chunk('R3', 'CHUNK', {}))
    m.define_chunks(actr.Chunk('R4', 'CHUNK', {}))
    m.define_chunks(actr.Chunk('R5', 'CHUNK', {}))
    m.define_chunks(actr.Chunk('R6', 'CHUNK', {}))
    m.define_chunks(actr.Chunk('R7', 'CHUNK', {}))
    m.define_chunks(actr.Chunk('R8', 'CHUNK', {}))
    m.add_chunk('ZERO', 'NUMBER', **{'VISUAL-REP': actr.String('0')})
    m.add_chunk('ONE', 'NUMBER', **{'VISUAL-REP': actr.String('1')})
    m.add_chunk('TWO', 'NUMBER', **{'VISUAL-REP': actr.String('2')})
    m.add_chunk('THREE', 'NUMBER', **{'VISUAL-REP': actr.String('3')})
    m.add_chunk('ADD', 'CHUNK', **{})
    m.add_chunk('SUBTRACT', 'CHUNK', **{})
    m.add_chunk('SKY', 'CHUNK', **{})
    m.add_chunk('GRASS', 'CHUNK', **{})
    m.add_chunk('ROSE', 'CHUNK', **{})
    m.add_chunk('1-1=0', 'MATH-FACT', **{'ARG1': 'ONE', 'ARG2': 'ONE', 'RESULT': 'ZERO', 'OPERATOR': 'SUBTRACT'})
    m.add_chunk('2-1=1', 'MATH-FACT', **{'ARG1': 'TWO', 'ARG2': 'ONE', 'RESULT': 'ONE', 'OPERATOR': 'SUBTRACT'})
    m.add_chunk('3-2=1', 'MATH-FACT', **{'ARG1': 'THREE', 'ARG2': 'TWO', 'RESULT': 'ONE', 'OPERATOR': 'SUBTRACT'})
    m.add_chunk('1+1=2', 'MATH-FACT', **{'ARG1': 'ONE', 'ARG2': 'ONE', 'RESULT': 'TWO', 'OPERATOR': 'ADD'})
    m.add_chunk('1+2=3', 'MATH-FACT', **{'ARG1': 'ONE', 'ARG2': 'TWO', 'RESULT': 'THREE', 'OPERATOR': 'ADD'})
    m.add_chunk('V1', 'SIMPLE-VALUE', **{'RESULT': actr.String('true')})
    m.add_chunk('V2', 'SIMPLE-VALUE', **{'RESULT': actr.String('false')})
    m.add_chunk('V3', 'SIMPLE-VALUE', **{'RESULT': None})
    m.add_chunk('F1', 'SIMPLE-FACT', **{'ITEM': 'SKY', 'ATTRIBUTE': 'BLUE'})
    m.add_chunk('F2', 'SIMPLE-FACT', **{'ITEM': 'ROSE', 'ATTRIBUTE': 'RED'})
    m.add_chunk('F3', 'SIMPLE-FACT', **{'ITEM': 'GRASS', 'ATTRIBUTE': 'GREEN'})
    m.add_chunk('G1', 'TASK-STATE', **{'STATE': 'R8'})
    m.add_chunk('OLD-CONTEXT', 'CONTEXT', **{'VAL1': 'ONE', 'VAL2': 'TWO', 'VAL3': 'ADD', 'GOAL': 'G1'})
    m.set_base_levels([('V1', 1, -1500), ('V2', 1, -1500), ('V3', 1, -1500)])
    m.declare_buffer_usage('IMAGINAL', 'CONTEXT', ':ALL')
    m.add_production(
        '''(P P1 ?GOAL> BUFFER EMPTY ==> +GOAL> ISA TASK-STATE STATE R2 +RETRIEVAL> ISA SIMPLE-VALUE - RESULT NIL)'''
    )
    m.add_production(
        '''(P P2 =GOAL> ISA TASK-STATE STATE R2 =RETRIEVAL> ==> =GOAL> STATE R3 +RETRIEVAL> ISA SIMPLE-VALUE)'''
    )
    m.add_production(
        '''(P P3 =GOAL> ISA TASK-STATE STATE R3 =RETRIEVAL> ISA SIMPLE-VALUE ==> =GOAL> STATE R4 +RETRIEVAL> ISA SIMPLE-FACT ATTRIBUTE PINK)'''
    )
    m.add_production(
        '''(P P4 =GOAL> ISA TASK-STATE STATE R4 ?RETRIEVAL> STATE FREE ==> =GOAL> STATE R5 +RETRIEVAL> ISA SIMPLE-FACT ATTRIBUTE BLACK)'''
    )
    m.add_production(
        '''(P P5 =GOAL> ISA TASK-STATE STATE R5 ?IMAGINAL> STATE FREE =RETRIEVAL> ==> =GOAL> STATE R6 +IMAGINAL> ISA CONTEXT VAL1 ONE VAL2 TWO +RETRIEVAL> ISA MATH-FACT - ARG1 NIL - ARG2 NIL - RESULT NIL)'''
    )
    m.add_production(
        '''(P P6 =GOAL> ISA TASK-STATE STATE R6 =IMAGINAL> ISA CONTEXT ==> +RETRIEVAL> ISA MATH-FACT - ARG1 NIL - ARG2 NIL - RESULT NIL =GOAL> STATE R7 =IMAGINAL> VAL3 ADD)'''
    )
    m.add_production(
        '''(P P7 =GOAL> ISA TASK-STATE STATE R7 =IMAGINAL> =RETRIEVAL> ==> =GOAL> STATE R8 =IMAGINAL> GOAL =GOAL)'''
    )
    m.add_production(
        '''(P P8 =GOAL> ISA TASK-STATE STATE R8 ==> +GOAL> -IMAGINAL>)'''
    )
    return m
