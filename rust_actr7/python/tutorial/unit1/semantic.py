"""Python port of ACT-R:tutorial;unit1;semantic.lisp."""

from __future__ import annotations

import rust_actr.actr as actr

LOGICAL_PATH = 'ACT-R:tutorial;unit1;semantic.lisp'
MODEL_NAME = 'SEMANTIC'
DEFAULT_RUN_TIME = 10.0


def build(seed=(100, 0)) -> actr.Model:
    m = actr.Model('SEMANTIC', seed=seed)
    m.sgp(':ESC', True)
    m.sgp(':LF', 0.05)
    m.chunk_type('PROPERTY', 'OBJECT', 'ATTRIBUTE', 'VALUE')
    m.chunk_type('IS-MEMBER', 'OBJECT', 'CATEGORY', 'JUDGMENT')
    m.add_dm('SHARK')
    m.add_dm('DANGEROUS')
    m.add_dm('LOCOMOTION')
    m.add_dm('SWIMMING')
    m.add_dm('FISH')
    m.add_dm('SALMON')
    m.add_dm('EDIBLE')
    m.add_dm('BREATHE')
    m.add_dm('GILLS')
    m.add_dm('ANIMAL')
    m.add_dm('MOVES')
    m.add_dm('SKIN')
    m.add_dm('CANARY')
    m.add_dm('COLOR')
    m.add_dm('SINGS')
    m.add_dm('BIRD')
    m.add_dm('OSTRICH')
    m.add_dm('FLIES')
    m.add_dm('HEIGHT')
    m.add_dm('TALL')
    m.add_dm('WINGS')
    m.add_dm('FLYING')
    m.add_dm('TRUE')
    m.add_dm('FALSE')
    m.add_chunk('P1', 'PROPERTY', **{'OBJECT': 'SHARK', 'ATTRIBUTE': 'DANGEROUS', 'VALUE': 'TRUE'})
    m.add_chunk('P2', 'PROPERTY', **{'OBJECT': 'SHARK', 'ATTRIBUTE': 'LOCOMOTION', 'VALUE': 'SWIMMING'})
    m.add_chunk('P3', 'PROPERTY', **{'OBJECT': 'SHARK', 'ATTRIBUTE': 'CATEGORY', 'VALUE': 'FISH'})
    m.add_chunk('P4', 'PROPERTY', **{'OBJECT': 'SALMON', 'ATTRIBUTE': 'EDIBLE', 'VALUE': 'TRUE'})
    m.add_chunk('P5', 'PROPERTY', **{'OBJECT': 'SALMON', 'ATTRIBUTE': 'LOCOMOTION', 'VALUE': 'SWIMMING'})
    m.add_chunk('P6', 'PROPERTY', **{'OBJECT': 'SALMON', 'ATTRIBUTE': 'CATEGORY', 'VALUE': 'FISH'})
    m.add_chunk('P7', 'PROPERTY', **{'OBJECT': 'FISH', 'ATTRIBUTE': 'BREATHE', 'VALUE': 'GILLS'})
    m.add_chunk('P8', 'PROPERTY', **{'OBJECT': 'FISH', 'ATTRIBUTE': 'LOCOMOTION', 'VALUE': 'SWIMMING'})
    m.add_chunk('P9', 'PROPERTY', **{'OBJECT': 'FISH', 'ATTRIBUTE': 'CATEGORY', 'VALUE': 'ANIMAL'})
    m.add_chunk('P10', 'PROPERTY', **{'OBJECT': 'ANIMAL', 'ATTRIBUTE': 'MOVES', 'VALUE': 'TRUE'})
    m.add_chunk('P11', 'PROPERTY', **{'OBJECT': 'ANIMAL', 'ATTRIBUTE': 'SKIN', 'VALUE': 'TRUE'})
    m.add_chunk('P12', 'PROPERTY', **{'OBJECT': 'CANARY', 'ATTRIBUTE': 'COLOR', 'VALUE': 'YELLOW'})
    m.add_chunk('P13', 'PROPERTY', **{'OBJECT': 'CANARY', 'ATTRIBUTE': 'SINGS', 'VALUE': 'TRUE'})
    m.add_chunk('P14', 'PROPERTY', **{'OBJECT': 'CANARY', 'ATTRIBUTE': 'CATEGORY', 'VALUE': 'BIRD'})
    m.add_chunk('P15', 'PROPERTY', **{'OBJECT': 'OSTRICH', 'ATTRIBUTE': 'FLIES', 'VALUE': 'FALSE'})
    m.add_chunk('P16', 'PROPERTY', **{'OBJECT': 'OSTRICH', 'ATTRIBUTE': 'HEIGHT', 'VALUE': 'TALL'})
    m.add_chunk('P17', 'PROPERTY', **{'OBJECT': 'OSTRICH', 'ATTRIBUTE': 'CATEGORY', 'VALUE': 'BIRD'})
    m.add_chunk('P18', 'PROPERTY', **{'OBJECT': 'BIRD', 'ATTRIBUTE': 'WINGS', 'VALUE': 'TRUE'})
    m.add_chunk('P19', 'PROPERTY', **{'OBJECT': 'BIRD', 'ATTRIBUTE': 'LOCOMOTION', 'VALUE': 'FLYING'})
    m.add_chunk('P20', 'PROPERTY', **{'OBJECT': 'BIRD', 'ATTRIBUTE': 'CATEGORY', 'VALUE': 'ANIMAL'})
    m.goal_focus(isa='IS-MEMBER', **{'OBJECT': 'CANARY', 'CATEGORY': 'BIRD'})
    m.add_production(
        '''(P INITIAL-RETRIEVE =GOAL> ISA IS-MEMBER OBJECT =OBJ CATEGORY =CAT JUDGMENT NIL ==> =GOAL> JUDGMENT PENDING +RETRIEVAL> ISA PROPERTY OBJECT =OBJ ATTRIBUTE CATEGORY)'''
    )
    m.add_production(
        '''(P DIRECT-VERIFY =GOAL> ISA IS-MEMBER OBJECT =OBJ CATEGORY =CAT JUDGMENT PENDING =RETRIEVAL> ISA PROPERTY OBJECT =OBJ ATTRIBUTE CATEGORY VALUE =CAT ==> =GOAL> JUDGMENT YES)'''
    )
    m.add_production(
        '''(P CHAIN-CATEGORY =GOAL> ISA IS-MEMBER OBJECT =OBJ1 CATEGORY =CAT JUDGMENT PENDING =RETRIEVAL> ISA PROPERTY OBJECT =OBJ1 ATTRIBUTE CATEGORY VALUE =OBJ2 - VALUE =CAT ==> =GOAL> OBJECT =OBJ2 +RETRIEVAL> ISA PROPERTY OBJECT =OBJ2 ATTRIBUTE CATEGORY)'''
    )
    m.add_production(
        '''(P FAIL =GOAL> ISA IS-MEMBER OBJECT =OBJ1 CATEGORY =CAT JUDGMENT PENDING ?RETRIEVAL> BUFFER FAILURE ==> =GOAL> JUDGMENT NO)'''
    )
    return m


def main(argv=None) -> None:
    import sys

    argv = list(sys.argv[1:] if argv is None else argv)
    seconds = float(argv[0]) if argv else DEFAULT_RUN_TIME
    print(build().run(seconds), end="")


if __name__ == "__main__":
    main()
