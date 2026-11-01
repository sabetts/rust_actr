"""Python port of ACT-R:tutorial;unit5;1hit-blackjack-model.lisp."""

from __future__ import annotations

import rust_actr as actr

LOGICAL_PATH = 'ACT-R:tutorial;unit5;1hit-blackjack-model.lisp'
MODEL_NAME = '1-HIT-MODEL'


def build(seed=(100, 0)) -> actr.Model:
    m = actr.Model('1-HIT-MODEL', seed=seed)
    m.sgp(':ESC', True)
    m.sgp(':BLL', 0.5)
    m.sgp(':OL', True)
    m.sgp(':SIM-HOOK', actr.String('1hit-bj-number-sims'))
    m.sgp(':CACHE-SIM-HOOK-RESULTS', True)
    m.sgp(':ER', True)
    m.sgp(':LF', 0)
    m.sgp(':V', None)
    m.sgp(':ANS', 0.2)
    m.sgp(':MP', 10.0)
    m.sgp(':RT', -60)
    m.chunk_type('GAME-STATE', 'MC1', 'MC2', 'MC3', 'MSTART', 'MTOT', 'MRESULT', 'OC1', 'OC2', 'OC3', 'OSTART', 'OTOT', 'ORESULT', 'STATE')
    m.chunk_type('LEARNED-INFO', 'MC1', 'ACTION')
    m.declare_buffer_usage('GOAL', 'GAME-STATE', ':ALL')
    m.define_chunks('WIN')
    m.define_chunks('LOSE')
    m.define_chunks('BUST')
    m.define_chunks('RETRIEVING')
    m.define_chunks('START')
    m.define_chunks('RESULTS')
    m.form('(INSTALL-DEVICE (QUOTE ("motor" "keyboard")))')
    m.add_production(
        '''(P START =GOAL> ISA GAME-STATE STATE START MC1 =C ==> =GOAL> STATE RETRIEVING +RETRIEVAL> ISA LEARNED-INFO MC1 =C - ACTION NIL)'''
    )
    m.add_production(
        '''(P CANT-REMEMBER-GAME =GOAL> ISA GAME-STATE STATE RETRIEVING ?RETRIEVAL> BUFFER FAILURE ?MANUAL> STATE FREE ==> =GOAL> STATE NIL +MANUAL> CMD PRESS-KEY KEY "s")'''
    )
    m.add_production(
        '''(P REMEMBER-GAME =GOAL> ISA GAME-STATE STATE RETRIEVING =RETRIEVAL> ISA LEARNED-INFO ACTION =ACT ?MANUAL> STATE FREE ==> =GOAL> STATE NIL +MANUAL> CMD PRESS-KEY KEY =ACT @RETRIEVAL>)'''
    )
    m.add_production(
        '''(P RESULTS-SHOULD-HIT =GOAL> ISA GAME-STATE STATE RESULTS MRESULT =OUTCOME MC1 =C ?IMAGINAL> STATE FREE ==> !OUTPUT! (I =OUTCOME) =GOAL> STATE NIL +IMAGINAL> MC1 =C ACTION "h")'''
    )
    m.spp('RESULTS-SHOULD-HIT', **{'U': 10})
    m.add_production(
        '''(P RESULTS-SHOULD-STAY =GOAL> ISA GAME-STATE STATE RESULTS MRESULT =OUTCOME MC1 =C ?IMAGINAL> STATE FREE ==> !OUTPUT! (I =OUTCOME) =GOAL> STATE NIL +IMAGINAL> MC1 =C ACTION "s")'''
    )
    m.add_production(
        '''(P CLEAR-NEW-IMAGINAL-CHUNK ?IMAGINAL> STATE FREE BUFFER FULL ==> -IMAGINAL>)'''
    )
    return m
