"""Python port of ACT-R:tutorial;unit3;subitize-model.lisp."""

from __future__ import annotations

import rust_actr as actr

LOGICAL_PATH = 'ACT-R:tutorial;unit3;subitize-model.lisp'
MODEL_NAME = 'SUBITIZE'


def build(seed=(100, 0)) -> actr.Model:
    m = actr.Model('SUBITIZE', seed=seed)
    m.sgp(':V', True)
    m.sgp(':SHOW-FOCUS', True)
    m.sgp(':VISUAL-NUM-FINSTS', 10)
    m.sgp(':VISUAL-FINST-SPAN', 10)
    m.chunk_type('COUNT', 'COUNT', 'STEP')
    m.chunk_type('NUMBER', 'NUMBER', 'NEXT', 'VOCAL-REP')
    m.add_chunk('ZERO', 'NUMBER', **{'NUMBER': 'ZERO', 'NEXT': 'ONE', 'VOCAL-REP': actr.String('zero')})
    m.add_chunk('ONE', 'NUMBER', **{'NUMBER': 'ONE', 'NEXT': 'TWO', 'VOCAL-REP': actr.String('one')})
    m.add_chunk('TWO', 'NUMBER', **{'NUMBER': 'TWO', 'NEXT': 'THREE', 'VOCAL-REP': actr.String('two')})
    m.add_chunk('THREE', 'NUMBER', **{'NUMBER': 'THREE', 'NEXT': 'FOUR', 'VOCAL-REP': actr.String('three')})
    m.add_chunk('FOUR', 'NUMBER', **{'NUMBER': 'FOUR', 'NEXT': 'FIVE', 'VOCAL-REP': actr.String('four')})
    m.add_chunk('FIVE', 'NUMBER', **{'NUMBER': 'FIVE', 'NEXT': 'SIX', 'VOCAL-REP': actr.String('five')})
    m.add_chunk('SIX', 'NUMBER', **{'NUMBER': 'SIX', 'NEXT': 'SEVEN', 'VOCAL-REP': actr.String('six')})
    m.add_chunk('SEVEN', 'NUMBER', **{'NUMBER': 'SEVEN', 'NEXT': 'EIGHT', 'VOCAL-REP': actr.String('seven')})
    m.add_chunk('EIGHT', 'NUMBER', **{'NUMBER': 'EIGHT', 'NEXT': 'NINE', 'VOCAL-REP': actr.String('eight')})
    m.add_chunk('NINE', 'NUMBER', **{'NUMBER': 'NINE', 'NEXT': 'TEN', 'VOCAL-REP': actr.String('nine')})
    m.add_chunk('TEN', 'NUMBER', **{'NUMBER': 'TEN', 'NEXT': 'ELEVEN', 'VOCAL-REP': actr.String('ten')})
    m.add_chunk('ELEVEN', 'NUMBER', **{'NUMBER': 'ELEVEN'})
    m.add_dm('START')
    m.goal_focus(isa='COUNT', **{'STEP': 'START'})
    return m
