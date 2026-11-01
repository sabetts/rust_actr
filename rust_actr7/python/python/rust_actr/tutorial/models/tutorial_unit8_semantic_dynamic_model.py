"""Python port of ACT-R:tutorial;unit8;semantic-dynamic-model.lisp."""

from __future__ import annotations

import rust_actr as actr

LOGICAL_PATH = 'ACT-R:tutorial;unit8;semantic-dynamic-model.lisp'
MODEL_NAME = 'SEMANTIC'


def build(seed=(100, 0)) -> actr.Model:
    m = actr.Model('SEMANTIC', seed=seed)
    m.sgp(':ESC', True)
    m.sgp(':LF', 0.05)
    m.chunk_type('PROPERTY', 'OBJECT', 'CATEGORY', 'DANGEROUS', 'LOCOMOTION', 'EDIBLE', 'BREATHE', 'MOVES', 'SKIN', 'COLOR', 'SINGS', 'FLIES', 'HEIGHT', 'WINGS')
    m.chunk_type('TEST-ATTRIBUTE', 'OBJECT', 'ORIGINAL', 'ATTRIBUTE', 'VALUE', 'STATE', 'ANSWER')
    m.define_chunks('SHARK')
    m.define_chunks('SALMON')
    m.define_chunks('FISH')
    m.define_chunks('ANIMAL')
    m.define_chunks('CANARY')
    m.define_chunks('OSTRICH')
    m.define_chunks('BIRD')
    m.define_chunks('DANGEROUS')
    m.define_chunks('LOCOMOTION')
    m.define_chunks('CATEGORY')
    m.define_chunks('EDIBLE')
    m.define_chunks('BREATHE')
    m.define_chunks('MOVES')
    m.define_chunks('SKIN')
    m.define_chunks('COLOR')
    m.define_chunks('SINGS')
    m.define_chunks('FLIES')
    m.define_chunks('HEIGHT')
    m.define_chunks('WINGS')
    m.define_chunks('GILLS')
    m.define_chunks('SWIMMING')
    m.define_chunks('TALL')
    m.define_chunks('FLYING')
    m.define_chunks('TRUE')
    m.define_chunks('FALSE')
    m.define_chunks('YES')
    m.define_chunks('NO')
    m.define_chunks('DONT-KNOW')
    m.define_chunks('ATTR-CHECK')
    m.define_chunks('RECORD')
    m.define_chunks('CREATE')
    m.define_chunks('DONE')
    m.define_chunks('CHECK-CATEGORY')
    m.add_chunk('P1', 'PROPERTY', **{'OBJECT': 'SHARK', 'DANGEROUS': 'TRUE'})
    m.add_chunk('P2', 'PROPERTY', **{'OBJECT': 'SHARK', 'LOCOMOTION': 'SWIMMING'})
    m.add_chunk('P3', 'PROPERTY', **{'OBJECT': 'SHARK', 'CATEGORY': 'FISH'})
    m.add_chunk('P4', 'PROPERTY', **{'OBJECT': 'SALMON', 'EDIBLE': 'TRUE'})
    m.add_chunk('P5', 'PROPERTY', **{'OBJECT': 'SALMON', 'LOCOMOTION': 'SWIMMING'})
    m.add_chunk('P6', 'PROPERTY', **{'OBJECT': 'SALMON', 'CATEGORY': 'FISH'})
    m.add_chunk('P7', 'PROPERTY', **{'OBJECT': 'FISH', 'BREATHE': 'GILLS'})
    m.add_chunk('P8', 'PROPERTY', **{'OBJECT': 'FISH', 'LOCOMOTION': 'SWIMMING'})
    m.add_chunk('P9', 'PROPERTY', **{'OBJECT': 'FISH', 'CATEGORY': 'ANIMAL'})
    m.add_chunk('P10', 'PROPERTY', **{'OBJECT': 'ANIMAL', 'MOVES': 'TRUE'})
    m.add_chunk('P11', 'PROPERTY', **{'OBJECT': 'ANIMAL', 'SKIN': 'TRUE'})
    m.add_chunk('P12', 'PROPERTY', **{'OBJECT': 'CANARY', 'COLOR': 'YELLOW'})
    m.add_chunk('P13', 'PROPERTY', **{'OBJECT': 'CANARY', 'SINGS': 'TRUE'})
    m.add_chunk('P14', 'PROPERTY', **{'OBJECT': 'CANARY', 'CATEGORY': 'BIRD'})
    m.add_chunk('P15', 'PROPERTY', **{'OBJECT': 'OSTRICH', 'FLIES': 'FALSE'})
    m.add_chunk('P16', 'PROPERTY', **{'OBJECT': 'OSTRICH', 'HEIGHT': 'TALL'})
    m.add_chunk('P17', 'PROPERTY', **{'OBJECT': 'OSTRICH', 'CATEGORY': 'BIRD'})
    m.add_chunk('P18', 'PROPERTY', **{'OBJECT': 'BIRD', 'WINGS': 'TRUE'})
    m.add_chunk('P19', 'PROPERTY', **{'OBJECT': 'BIRD', 'LOCOMOTION': 'FLYING'})
    m.add_chunk('P20', 'PROPERTY', **{'OBJECT': 'BIRD', 'CATEGORY': 'ANIMAL'})
    m.add_chunk('P21', 'PROPERTY', **{'OBJECT': 'BIRD', 'FLIES': 'TRUE'})
    m.define_chunks(actr.Chunk('G1', 'TEST-ATTRIBUTE', {'OBJECT': 'CANARY', 'ATTRIBUTE': 'CATEGORY', 'VALUE': 'BIRD'}))
    m.define_chunks(actr.Chunk('G2', 'TEST-ATTRIBUTE', {'OBJECT': 'CANARY', 'ATTRIBUTE': 'WINGS', 'VALUE': 'TRUE'}))
    m.define_chunks(actr.Chunk('G3', 'TEST-ATTRIBUTE', {'OBJECT': 'CANARY', 'ATTRIBUTE': 'DANGEROUS', 'VALUE': 'TRUE'}))
    m.declare_buffer_usage('GOAL', 'TEST-ATTRIBUTE', ':ALL')
    m.add_production(
        '''(P RETRIEVE-ATTRIBUTE =GOAL> ISA TEST-ATTRIBUTE OBJECT =OBJ ATTRIBUTE =ATTR VALUE =VAL STATE NIL ==> =GOAL> STATE ATTR-CHECK +RETRIEVAL> ISA PROPERTY OBJECT =OBJ - =ATTR NIL)'''
    )
    m.add_production(
        '''(P DIRECT-VERIFY =GOAL> ISA TEST-ATTRIBUTE ORIGINAL NIL OBJECT =OBJ ATTRIBUTE =ATTR VALUE =VAL STATE ATTR-CHECK =RETRIEVAL> ISA PROPERTY OBJECT =OBJ =ATTR =VAL ==> =GOAL> ANSWER YES STATE DONE)'''
    )
    m.add_production(
        '''(P INDIRECT-VERIFY =GOAL> ISA TEST-ATTRIBUTE ORIGINAL =ORI OBJECT =OBJ ATTRIBUTE =ATTR VALUE =VAL STATE ATTR-CHECK =RETRIEVAL> ISA PROPERTY OBJECT =OBJ =ATTR =VAL ==> =GOAL> OBJECT =ORI ANSWER YES STATE RECORD)'''
    )
    m.add_production(
        '''(P RECORD-RESULT =GOAL> ISA TEST-ATTRIBUTE OBJECT =OBJ ATTRIBUTE =ATTR VALUE =VAL STATE RECORD ?IMAGINAL> STATE FREE ==> +IMAGINAL> OBJECT =OBJ =ATTR =VAL =GOAL> ANSWER YES STATE CREATE)'''
    )
    m.add_production(
        '''(P CREATE =GOAL> ISA TEST-ATTRIBUTE STATE CREATE ?IMAGINAL> STATE FREE BUFFER FULL ==> -IMAGINAL> =GOAL> STATE DONE)'''
    )
    m.add_production(
        '''(P DISPROVE =GOAL> ISA TEST-ATTRIBUTE OBJECT =OBJ ATTRIBUTE =ATTR VALUE =VAL STATE ATTR-CHECK =RETRIEVAL> ISA PROPERTY OBJECT =OBJ - =ATTR =VAL ==> =GOAL> ANSWER NO STATE DONE)'''
    )
    m.add_production(
        '''(P MISSING-ATTR =GOAL> ISA TEST-ATTRIBUTE OBJECT =OBJ STATE ATTR-CHECK ?RETRIEVAL> BUFFER FAILURE ==> =GOAL> STATE CHECK-CATEGORY +RETRIEVAL> ISA PROPERTY OBJECT =OBJ - CATEGORY NIL)'''
    )
    m.add_production(
        '''(P CHAIN-FIRST-CATEGORY =GOAL> ISA TEST-ATTRIBUTE ORIGINAL NIL OBJECT =OBJ STATE CHECK-CATEGORY =RETRIEVAL> ISA PROPERTY OBJECT =OBJ CATEGORY =OBJ2 ==> =GOAL> ORIGINAL =OBJ OBJECT =OBJ2 STATE NIL)'''
    )
    m.add_production(
        '''(P CHAIN-SUBSEQUENT-CATEGORY =GOAL> ISA TEST-ATTRIBUTE - ORIGINAL NIL OBJECT =OBJ STATE CHECK-CATEGORY =RETRIEVAL> ISA PROPERTY OBJECT =OBJ CATEGORY =OBJ2 ==> =GOAL> OBJECT =OBJ2 STATE NIL)'''
    )
    m.add_production(
        '''(P FAILED-TO-FIND-CATEGORY =GOAL> ISA TEST-ATTRIBUTE STATE CHECK-CATEGORY ?RETRIEVAL> BUFFER FAILURE ==> =GOAL> ANSWER DONT-KNOW STATE DONE)'''
    )
    m.add_production(
        '''(P RESPOND =GOAL> ISA TEST-ATTRIBUTE STATE DONE OBJECT =OBJ ATTRIBUTE =ATTR VALUE =VAL ANSWER =ANS ==> !OUTPUT! (=OBJ =ATTR =VAL ANSWER IS =ANS) -GOAL>)'''
    )
    return m
