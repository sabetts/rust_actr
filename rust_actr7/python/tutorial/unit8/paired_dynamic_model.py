"""Python port of ACT-R:tutorial;unit8;paired-dynamic-model.lisp."""

from __future__ import annotations

import rust_actr.actr as actr

LOGICAL_PATH = 'ACT-R:tutorial;unit8;paired-dynamic-model.lisp'
MODEL_NAME = 'PAIRED-LEARNING-DYNAMIC'
DEFAULT_RUN_TIME = 10.0


def build(seed=(100, 0)) -> actr.Model:
    m = actr.Model('PAIRED-LEARNING-DYNAMIC', seed=seed)
    m.sgp(':ESC', True)
    m.sgp(':V', True)
    m.sgp(':PCT', True)
    m.sgp(':UL', True)
    m.sgp(':EPL', True)
    m.sgp(':RT', -1.7)
    m.sgp(':LF', 0.2)
    m.sgp(':ANS', 0.4)
    m.sgp(':EGS', 0.1)
    m.sgp(':BLL', 0.5)
    m.sgp(':TRACE-DETAIL', 'LOW')
    m.sgp(':IU', 5)
    m.sgp(':ALPHA', 0.2)
    m.sgp(':DO-NOT-HARVEST', 'IMAGINAL')
    m.chunk_type('OPERATOR', 'PRE', 'ACTION', 'ARG1', 'ARG2', 'POST', 'LABEL', 'REQUIRED', 'SLOT', 'SUCCESS', 'FAILURE')
    m.chunk_type('TASK', 'STATE', 'STEP', 'CONTEXT')
    m.define_chunks('STIMULUS-READ')
    m.define_chunks('READ')
    m.define_chunks('START')
    m.define_chunks('RECALLED')
    m.define_chunks('NUMBER')
    m.define_chunks('RETRIEVE')
    m.define_chunks('WAIT')
    m.define_chunks('RESPOND')
    m.define_chunks('TYPE')
    m.define_chunks('NEW-TRIAL')
    m.define_chunks('COMPLETE-TASK')
    m.define_chunks('READY')
    m.define_chunks('RETRIEVING-OPERATOR')
    m.define_chunks('PROCESS')
    m.define_chunks('RETRIEVING-RESULT')
    m.add_chunk('OP1', 'OPERATOR', **{'PRE': 'START', 'ACTION': 'READ', 'LABEL': 'WORD', 'POST': 'STIMULUS-READ'})
    m.add_chunk('OP2', 'OPERATOR', **{'PRE': 'STIMULUS-READ', 'ACTION': 'RETRIEVE', 'REQUIRED': 'WORD', 'LABEL': 'NUMBER', 'POST': 'RECALLED'})
    m.add_chunk('OP3', 'OPERATOR', **{'PRE': 'RECALLED', 'SLOT': 'NUMBER', 'SUCCESS': 'RESPOND', 'FAILURE': 'WAIT'})
    m.add_chunk('OP4', 'OPERATOR', **{'PRE': 'RESPOND', 'ACTION': 'TYPE', 'REQUIRED': 'NUMBER', 'POST': 'WAIT'})
    m.add_chunk('OP5', 'OPERATOR', **{'PRE': 'WAIT', 'ACTION': 'READ', 'LABEL': 'NUMBER', 'POST': 'NEW-TRIAL'})
    m.add_chunk('OP6', 'OPERATOR', **{'PRE': 'NEW-TRIAL', 'ACTION': 'COMPLETE-TASK', 'POST': 'START'})
    m.set_all_base_levels(1000)
    m.goal_focus(isa='TASK', **{'STATE': 'START', 'STEP': 'READY'})
    m.add_production(
        '''(P RETRIEVE-OPERATOR =GOAL> ISA TASK STATE =STATE STEP READY ==> +RETRIEVAL> ISA OPERATOR PRE =STATE =GOAL> STEP RETRIEVING-OPERATOR CONTEXT NIL)'''
    )
    m.add_production(
        '''(P READ-AND-CREATE =GOAL> ISA TASK STEP RETRIEVING-OPERATOR =RETRIEVAL> ISA OPERATOR ACTION READ LABEL =DESTINATION POST =STATE =VISUAL-LOCATION> ?VISUAL> STATE FREE ?IMAGINAL> BUFFER EMPTY STATE FREE ==> +IMAGINAL> +VISUAL> ISA MOVE-ATTENTION SCREEN-POS =VISUAL-LOCATION =GOAL> CONTEXT =DESTINATION STEP PROCESS STATE =STATE)'''
    )
    m.add_production(
        '''(P READ =GOAL> ISA TASK STEP RETRIEVING-OPERATOR =RETRIEVAL> ISA OPERATOR ACTION READ LABEL =DESTINATION POST =STATE =VISUAL-LOCATION> ?VISUAL> STATE FREE ?IMAGINAL> BUFFER FULL ==> +VISUAL> ISA MOVE-ATTENTION SCREEN-POS =VISUAL-LOCATION =GOAL> CONTEXT =DESTINATION STEP PROCESS STATE =STATE)'''
    )
    m.add_production(
        '''(P ENCODE =GOAL> ISA TASK STEP PROCESS CONTEXT =WHICH-SLOT =VISUAL> ISA TEXT VALUE =VAL =IMAGINAL> ?IMAGINAL> STATE FREE ==> *IMAGINAL> =WHICH-SLOT =VAL =GOAL> STEP READY)'''
    )
    m.add_production(
        '''(P RETRIEVE-ASSOCIATE =GOAL> ISA TASK STEP RETRIEVING-OPERATOR =IMAGINAL> =TARGET =STIMULUS =RETRIEVAL> ISA OPERATOR ACTION RETRIEVE REQUIRED =TARGET LABEL =OTHER POST =STATE ==> +RETRIEVAL> =TARGET =STIMULUS =GOAL> STEP RETRIEVING-RESULT CONTEXT =OTHER STATE =STATE)'''
    )
    m.add_production(
        '''(P RETRIEVAL-UNSUCCESSFUL =GOAL> ISA TASK STEP RETRIEVING-RESULT CONTEXT =DEST =IMAGINAL> ?RETRIEVAL> BUFFER FAILURE ?IMAGINAL> STATE FREE ==> *IMAGINAL> =DEST NIL =GOAL> STEP READY)'''
    )
    m.add_production(
        '''(P RETRIEVAL-SUCCESSFUL =GOAL> ISA TASK STEP RETRIEVING-RESULT CONTEXT =DEST =IMAGINAL> =RETRIEVAL> =DEST =VAL ?IMAGINAL> STATE FREE ==> *IMAGINAL> =DEST =VAL =GOAL> STEP READY)'''
    )
    m.add_production(
        '''(P UNSUCCESSFUL-TEST =GOAL> ISA TASK STEP RETRIEVING-OPERATOR =IMAGINAL> =DEST NIL =RETRIEVAL> ISA OPERATOR SLOT =DEST FAILURE =STATE ?IMAGINAL> STATE FREE ==> =GOAL> STATE =STATE STEP READY)'''
    )
    m.add_production(
        '''(P SUCCESSFUL-TEST =GOAL> ISA TASK STEP RETRIEVING-OPERATOR =IMAGINAL> =DEST =VAL =RETRIEVAL> ISA OPERATOR SLOT =DEST SUCCESS =STATE ?IMAGINAL> STATE FREE ==> =GOAL> STATE =STATE STEP READY)'''
    )
    m.add_production(
        '''(P TYPE =GOAL> ISA TASK STEP RETRIEVING-OPERATOR =IMAGINAL> =SLOT =VAL =RETRIEVAL> ISA OPERATOR ACTION TYPE REQUIRED =SLOT POST =STATE ?MANUAL> STATE FREE ==> +MANUAL> CMD PRESS-KEY KEY =VAL =GOAL> STATE =STATE STEP READY)'''
    )
    m.add_production(
        '''(P COMPLETE-TASK =GOAL> ISA TASK STEP RETRIEVING-OPERATOR =RETRIEVAL> ISA OPERATOR ACTION COMPLETE-TASK POST =STATE ?IMAGINAL> STATE FREE ==> -IMAGINAL> +GOAL> ISA TASK STATE =STATE STEP READY)'''
    )
    m.spp('COMPLETE-TASK', **{'REWARD': 20})
    return m


def main(argv=None) -> None:
    import sys

    argv = list(sys.argv[1:] if argv is None else argv)
    seconds = float(argv[0]) if argv else DEFAULT_RUN_TIME
    print(build().run(seconds), end="")


if __name__ == "__main__":
    main()
