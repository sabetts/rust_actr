"""Python port of ACT-R:tutorial;unit5;fan-no-pm-model.lisp."""

from __future__ import annotations

import rust_actr as actr

LOGICAL_PATH = 'ACT-R:tutorial;unit5;fan-no-pm-model.lisp'
MODEL_NAME = 'FAN'


def build(seed=(100, 0)) -> actr.Model:
    m = actr.Model('FAN', seed=seed)
    m.sgp(':V', True)
    m.sgp(':ACT', None)
    m.sgp(':ESC', True)
    m.sgp(':LF', 0.63)
    m.sgp(':MAS', 1.6)
    m.sgp(':GA', 1.0)
    m.sgp(':STYLE-WARNINGS', None)
    m.chunk_type('COMPREHEND-SENTENCE', 'RELATION', 'ARG1', 'ARG2')
    m.chunk_type('MEANING', 'WORD')
    m.add_chunk('P1', 'COMPREHEND-SENTENCE', **{'RELATION': 'IN', 'ARG1': 'HIPPIE', 'ARG2': 'PARK'})
    m.add_chunk('P2', 'COMPREHEND-SENTENCE', **{'RELATION': 'IN', 'ARG1': 'HIPPIE', 'ARG2': 'CHURCH'})
    m.add_chunk('P3', 'COMPREHEND-SENTENCE', **{'RELATION': 'IN', 'ARG1': 'HIPPIE', 'ARG2': 'BANK'})
    m.add_chunk('P4', 'COMPREHEND-SENTENCE', **{'RELATION': 'IN', 'ARG1': 'CAPTAIN', 'ARG2': 'PARK'})
    m.add_chunk('P5', 'COMPREHEND-SENTENCE', **{'RELATION': 'IN', 'ARG1': 'CAPTAIN', 'ARG2': 'CAVE'})
    m.add_chunk('P6', 'COMPREHEND-SENTENCE', **{'RELATION': 'IN', 'ARG1': 'DEBUTANTE', 'ARG2': 'BANK'})
    m.add_chunk('P7', 'COMPREHEND-SENTENCE', **{'RELATION': 'IN', 'ARG1': 'FIREMAN', 'ARG2': 'PARK'})
    m.add_chunk('P8', 'COMPREHEND-SENTENCE', **{'RELATION': 'IN', 'ARG1': 'GIANT', 'ARG2': 'BEACH'})
    m.add_chunk('P9', 'COMPREHEND-SENTENCE', **{'RELATION': 'IN', 'ARG1': 'GIANT', 'ARG2': 'CASTLE'})
    m.add_chunk('P10', 'COMPREHEND-SENTENCE', **{'RELATION': 'IN', 'ARG1': 'GIANT', 'ARG2': 'DUNGEON'})
    m.add_chunk('P11', 'COMPREHEND-SENTENCE', **{'RELATION': 'IN', 'ARG1': 'EARL', 'ARG2': 'CASTLE'})
    m.add_chunk('P12', 'COMPREHEND-SENTENCE', **{'RELATION': 'IN', 'ARG1': 'EARL', 'ARG2': 'FOREST'})
    m.add_chunk('P13', 'COMPREHEND-SENTENCE', **{'RELATION': 'IN', 'ARG1': 'LAWYER', 'ARG2': 'STORE'})
    m.add_chunk('GUARD', 'MEANING', **{'WORD': actr.String('guard')})
    m.add_chunk('BEACH', 'MEANING', **{'WORD': actr.String('beach')})
    m.add_chunk('CASTLE', 'MEANING', **{'WORD': actr.String('castle')})
    m.add_chunk('DUNGEON', 'MEANING', **{'WORD': actr.String('dungeon')})
    m.add_chunk('EARL', 'MEANING', **{'WORD': actr.String('earl')})
    m.add_chunk('FOREST', 'MEANING', **{'WORD': actr.String('forest')})
    m.add_chunk('GIANT', 'MEANING', **{'WORD': actr.String('giant')})
    m.add_chunk('HIPPIE', 'MEANING', **{'WORD': actr.String('hippie')})
    m.add_chunk('PARK', 'MEANING', **{'WORD': actr.String('park')})
    m.add_chunk('CHURCH', 'MEANING', **{'WORD': actr.String('church')})
    m.add_chunk('CAPTAIN', 'MEANING', **{'WORD': actr.String('captain')})
    m.add_chunk('CAVE', 'MEANING', **{'WORD': actr.String('cave')})
    m.add_chunk('DEBUTANTE', 'MEANING', **{'WORD': actr.String('debutante')})
    m.add_chunk('BANK', 'MEANING', **{'WORD': actr.String('bank')})
    m.add_chunk('FIREMAN', 'MEANING', **{'WORD': actr.String('fireman')})
    m.add_chunk('LAWYER', 'MEANING', **{'WORD': actr.String('lawyer')})
    m.add_chunk('STORE', 'MEANING', **{'WORD': actr.String('store')})
    m.add_chunk('IN', 'MEANING', **{'WORD': actr.String('in')})
    m.set_base_levels([('GUARD', 10), ('BEACH', 10), ('CASTLE', 10), ('DUNGEON', 10), ('EARL', 10), ('FOREST', 10), ('HIPPIE', 10), ('PARK', 10), ('CHURCH', 10), ('BANK', 10), ('CAPTAIN', 10), ('CAVE', 10), ('GIANT', 10), ('DEBUTANTE', 10), ('FIREMAN', 10), ('LAWYER', 10), ('STORE', 10), ('IN', 10)])
    m.chunk_type('SENTENCE-GOAL', 'ARG1', 'ARG2', 'STATE')
    m.define_chunks(actr.Chunk('TEST', 'CHUNK', {}))
    m.define_chunks(actr.Chunk('HARVEST-PERSON', 'CHUNK', {}))
    m.define_chunks(actr.Chunk('HARVEST-LOCATION', 'CHUNK', {}))
    m.define_chunks(actr.Chunk('GET-RETRIEVAL', 'CHUNK', {}))
    m.goal_focus(isa='SENTENCE-GOAL', **{'STATE': 'TEST'})
    m.add_production(
        '''(P START =GOAL> ISA SENTENCE-GOAL ARG1 =PERSON STATE TEST ==> =GOAL> STATE HARVEST-PERSON +RETRIEVAL> ISA MEANING WORD =PERSON)'''
    )
    m.add_production(
        '''(P HARVEST-PERSON =GOAL> ISA SENTENCE-GOAL ARG2 =LOCATION STATE HARVEST-PERSON =RETRIEVAL> ==> =GOAL> ARG1 =RETRIEVAL STATE HARVEST-LOCATION +RETRIEVAL> ISA MEANING WORD =LOCATION)'''
    )
    m.add_production(
        '''(P HARVEST-LOCATION =GOAL> ISA SENTENCE-GOAL STATE HARVEST-LOCATION =RETRIEVAL> ISA MEANING ==> =GOAL> ARG2 =RETRIEVAL STATE GET-RETRIEVAL)'''
    )
    m.add_production(
        '''(P RETRIEVE-FROM-PERSON =GOAL> ISA SENTENCE-GOAL ARG1 =PERSON STATE GET-RETRIEVAL ==> =GOAL> STATE NIL +RETRIEVAL> ISA COMPREHEND-SENTENCE ARG1 =PERSON)'''
    )
    m.add_production(
        '''(P RETRIEVE-FROM-LOCATION =GOAL> ISA SENTENCE-GOAL ARG2 =LOCATION STATE GET-RETRIEVAL ==> =GOAL> STATE NIL +RETRIEVAL> ISA COMPREHEND-SENTENCE ARG2 =LOCATION)'''
    )
    m.add_production(
        '''(P RESPOND-YES =GOAL> ISA SENTENCE-GOAL ARG1 =PERSON ARG2 =LOCATION STATE NIL =RETRIEVAL> ISA COMPREHEND-SENTENCE ARG1 =PERSON ARG2 =LOCATION ==> =GOAL> STATE "k")'''
    )
    m.add_production(
        '''(P MISMATCH-PERSON-NO =GOAL> ISA SENTENCE-GOAL ARG1 =PERSON ARG2 =LOCATION STATE NIL =RETRIEVAL> ISA COMPREHEND-SENTENCE - ARG1 =PERSON ==> =GOAL> STATE "d")'''
    )
    m.add_production(
        '''(P MISMATCH-LOCATION-NO =GOAL> ISA SENTENCE-GOAL ARG1 =PERSON ARG2 =LOCATION STATE NIL =RETRIEVAL> ISA COMPREHEND-SENTENCE - ARG2 =LOCATION ==> =GOAL> STATE "d")'''
    )
    m.spp('MISMATCH-LOCATION-NO', **{'AT': 0.21})
    m.spp('MISMATCH-PERSON-NO', **{'AT': 0.21})
    m.spp('RESPOND-YES', **{'AT': 0.21})
    m.spp('START', **{'AT': 0.25})
    m.spp('HARVEST-PERSON', **{'AT': 0.285})
    return m
