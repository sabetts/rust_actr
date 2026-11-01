"""Python port of ACT-R:tutorial;unit1;addition.lisp."""

from __future__ import annotations

import rust_actr.actr as actr

LOGICAL_PATH = 'ACT-R:tutorial;unit1;addition.lisp'
MODEL_NAME = 'ADDITION'
DEFAULT_RUN_TIME = 10.0


def build(seed=(100, 0)) -> actr.Model:
    m = actr.Model('ADDITION', seed=seed)
    m.sgp(':ESC', True)
    m.sgp(':LF', 0.05)
    m.chunk_type('NUMBER', 'NUMBER', 'NEXT')
    m.chunk_type('ADD', 'ARG1', 'ARG2', 'SUM', 'COUNT')
    m.add_chunk('ZERO', 'NUMBER', **{'NUMBER': 'ZERO', 'NEXT': 'ONE'})
    m.add_chunk('ONE', 'NUMBER', **{'NUMBER': 'ONE', 'NEXT': 'TWO'})
    m.add_chunk('TWO', 'NUMBER', **{'NUMBER': 'TWO', 'NEXT': 'THREE'})
    m.add_chunk('THREE', 'NUMBER', **{'NUMBER': 'THREE', 'NEXT': 'FOUR'})
    m.add_chunk('FOUR', 'NUMBER', **{'NUMBER': 'FOUR', 'NEXT': 'FIVE'})
    m.add_chunk('FIVE', 'NUMBER', **{'NUMBER': 'FIVE', 'NEXT': 'SIX'})
    m.add_chunk('SIX', 'NUMBER', **{'NUMBER': 'SIX', 'NEXT': 'SEVEN'})
    m.add_chunk('SEVEN', 'NUMBER', **{'NUMBER': 'SEVEN', 'NEXT': 'EIGHT'})
    m.add_chunk('EIGHT', 'NUMBER', **{'NUMBER': 'EIGHT', 'NEXT': 'NINE'})
    m.add_chunk('NINE', 'NUMBER', **{'NUMBER': 'NINE', 'NEXT': 'TEN'})
    m.add_chunk('TEN', 'NUMBER', **{'NUMBER': 'TEN'})
    m.goal_focus(isa='ADD', **{'ARG1': 'FIVE', 'ARG2': 'TWO'})
    m.add_production(
        '''(P INITIALIZE-ADDITION =GOAL> ISA ADD ARG1 =NUM1 ARG2 =NUM2 SUM NIL ==> =GOAL> ISA ADD SUM =NUM1 COUNT ZERO +RETRIEVAL> ISA NUMBER NUMBER =NUM1)'''
    )
    m.add_production(
        '''(P TERMINATE-ADDITION =GOAL> ISA ADD COUNT =NUM ARG2 =NUM SUM =ANSWER =RETRIEVAL> ISA NUMBER NUMBER =ANSWER ==> =GOAL> ISA ADD COUNT NIL !OUTPUT! (=ANSWER))'''
    )
    m.add_production(
        '''(P INCREMENT-COUNT =GOAL> ISA ADD SUM =SUM COUNT =COUNT =RETRIEVAL> ISA NUMBER NUMBER =COUNT NEXT =NEWCOUNT ==> =GOAL> ISA ADD COUNT =NEWCOUNT +RETRIEVAL> ISA NUMBER NUMBER =SUM)'''
    )
    m.add_production(
        '''(P INCREMENT-SUM =GOAL> ISA ADD SUM =SUM COUNT =COUNT - ARG2 =COUNT =RETRIEVAL> ISA NUMBER NUMBER =SUM NEXT =NEWSUM ==> =GOAL> ISA ADD SUM =NEWSUM +RETRIEVAL> ISA NUMBER NUMBER =COUNT)'''
    )
    return m


def main(argv=None) -> None:
    import sys

    argv = list(sys.argv[1:] if argv is None else argv)
    seconds = float(argv[0]) if argv else DEFAULT_RUN_TIME
    print(build().run(seconds), end="")


if __name__ == "__main__":
    main()
