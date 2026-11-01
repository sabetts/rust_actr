"""Python port of ACT-R:tutorial;unit8;simple-dynamic-model.lisp."""

from __future__ import annotations

import rust_actr.actr as actr

LOGICAL_PATH = 'ACT-R:tutorial;unit8;simple-dynamic-model.lisp'
MODEL_NAME = 'DYNAMIC-INTRODUCTION'
DEFAULT_RUN_TIME = 10.0


def build(seed=(100, 0)) -> actr.Model:
    m = actr.Model('DYNAMIC-INTRODUCTION', seed=seed)
    m.sgp(':V', True)
    m.sgp(':TRACE-DETAIL', 'HIGH')
    m.sgp(':CST', True)
    m.sgp(':STYLE-WARNINGS', None)
    m.chunk_type('FACT', 'CONTEXT', 'DATA')
    m.chunk_type('STEP', 'STEP', 'DESTINATION')
    m.chunk_type('RESULT', 'DATA1', 'DATA2')
    m.add_chunk('FIRST', 'CHUNK', **{})
    m.add_chunk('SECOND', 'CHUNK', **{})
    m.add_chunk('DESTINATION', 'CHUNK', **{})
    m.add_chunk('DATA2', 'CHUNK', **{})
    m.add_chunk('DATA3', 'CHUNK', **{})
    m.add_chunk('DATA', 'CHUNK', **{})
    m.add_chunk('A', 'STEP', **{'STEP': 'FIRST', 'DESTINATION': 'DATA2'})
    m.add_chunk('B', 'STEP', **{'STEP': 'SECOND', 'DESTINATION': 'DATA3'})
    m.goal_focus(isa='FACT', **{'CONTEXT': 'DATA', 'DATA': 10})
    m.add_production(
        '''(P START =GOAL> ISA FACT CONTEXT =CONTEXT =CONTEXT =X ?IMAGINAL> STATE FREE BUFFER EMPTY ==> =GOAL> CONTEXT DESTINATION +IMAGINAL> ISA RESULT DATA1 =X +RETRIEVAL> ISA STEP STEP FIRST)'''
    )
    m.add_production(
        '''(P RETRIEVE-FIRST-STEP =GOAL> ISA FACT CONTEXT =SLOT DATA =X =IMAGINAL> ISA RESULT DATA1 =X =RETRIEVAL> ISA STEP =SLOT =TARGET STEP FIRST ==> =GOAL> DATA 11 =IMAGINAL> =TARGET =X +RETRIEVAL> ISA STEP STEP SECOND)'''
    )
    m.add_production(
        '''(P RETRIEVE-SECOND-STEP =GOAL> ISA FACT CONTEXT =SLOT DATA =X =IMAGINAL> =RETRIEVAL> ISA STEP =SLOT =TARGET STEP SECOND ==> =IMAGINAL> =TARGET =X)'''
    )
    return m


def main(argv=None) -> None:
    import sys

    argv = list(sys.argv[1:] if argv is None else argv)
    seconds = float(argv[0]) if argv else DEFAULT_RUN_TIME
    print(build().run(seconds), end="")


if __name__ == "__main__":
    main()
