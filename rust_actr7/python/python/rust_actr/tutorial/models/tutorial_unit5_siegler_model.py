"""Python port of ACT-R:tutorial;unit5;siegler-model.lisp."""

from __future__ import annotations

import rust_actr as actr

LOGICAL_PATH = 'ACT-R:tutorial;unit5;siegler-model.lisp'
MODEL_NAME = 'SIEGLER'


def build(seed=(100, 0)) -> actr.Model:
    m = actr.Model('SIEGLER', seed=seed)
    m.sgp(':RT', -0.45)
    m.sgp(':ESC', True)
    m.sgp(':V', None)
    m.sgp(':ACT', None)
    m.sgp(':ANS', 0.5)
    m.sgp(':MP', 16)
    m.chunk_type('PLUS-FACT', 'ADDEND1', 'ADDEND2', 'SUM')
    m.chunk_type('NUMBER', 'AURAL-REP', 'VOCAL-REP')
    m.add_chunk('ZERO', 'NUMBER', **{'AURAL-REP': 0, 'VOCAL-REP': actr.String('zero')})
    m.add_chunk('ONE', 'NUMBER', **{'AURAL-REP': 1, 'VOCAL-REP': actr.String('one')})
    m.add_chunk('TWO', 'NUMBER', **{'AURAL-REP': 2, 'VOCAL-REP': actr.String('two')})
    m.add_chunk('THREE', 'NUMBER', **{'AURAL-REP': 3, 'VOCAL-REP': actr.String('three')})
    m.add_chunk('FOUR', 'NUMBER', **{'AURAL-REP': 4, 'VOCAL-REP': actr.String('four')})
    m.add_chunk('FIVE', 'NUMBER', **{'AURAL-REP': 5, 'VOCAL-REP': actr.String('five')})
    m.add_chunk('SIX', 'NUMBER', **{'AURAL-REP': 6, 'VOCAL-REP': actr.String('six')})
    m.add_chunk('SEVEN', 'NUMBER', **{'AURAL-REP': 7, 'VOCAL-REP': actr.String('seven')})
    m.add_chunk('EIGHT', 'NUMBER', **{'AURAL-REP': 8, 'VOCAL-REP': actr.String('eight')})
    m.add_chunk('NINE', 'NUMBER', **{'AURAL-REP': 9, 'VOCAL-REP': actr.String('nine')})
    m.add_chunk('F00', 'PLUS-FACT', **{'ADDEND1': 'ZERO', 'ADDEND2': 'ZERO', 'SUM': 'ZERO'})
    m.add_chunk('F01', 'PLUS-FACT', **{'ADDEND1': 'ZERO', 'ADDEND2': 'ONE', 'SUM': 'ONE'})
    m.add_chunk('F02', 'PLUS-FACT', **{'ADDEND1': 'ZERO', 'ADDEND2': 'TWO', 'SUM': 'TWO'})
    m.add_chunk('F03', 'PLUS-FACT', **{'ADDEND1': 'ZERO', 'ADDEND2': 'THREE', 'SUM': 'THREE'})
    m.add_chunk('F04', 'PLUS-FACT', **{'ADDEND1': 'ZERO', 'ADDEND2': 'FOUR', 'SUM': 'FOUR'})
    m.add_chunk('F05', 'PLUS-FACT', **{'ADDEND1': 'ZERO', 'ADDEND2': 'FIVE', 'SUM': 'FIVE'})
    m.add_chunk('F10', 'PLUS-FACT', **{'ADDEND1': 'ONE', 'ADDEND2': 'ZERO', 'SUM': 'ONE'})
    m.add_chunk('F11', 'PLUS-FACT', **{'ADDEND1': 'ONE', 'ADDEND2': 'ONE', 'SUM': 'TWO'})
    m.add_chunk('F12', 'PLUS-FACT', **{'ADDEND1': 'ONE', 'ADDEND2': 'TWO', 'SUM': 'THREE'})
    m.add_chunk('F13', 'PLUS-FACT', **{'ADDEND1': 'ONE', 'ADDEND2': 'THREE', 'SUM': 'FOUR'})
    m.add_chunk('F14', 'PLUS-FACT', **{'ADDEND1': 'ONE', 'ADDEND2': 'FOUR', 'SUM': 'FIVE'})
    m.add_chunk('F15', 'PLUS-FACT', **{'ADDEND1': 'ONE', 'ADDEND2': 'FIVE', 'SUM': 'SIX'})
    m.add_chunk('F20', 'PLUS-FACT', **{'ADDEND1': 'TWO', 'ADDEND2': 'ZERO', 'SUM': 'TWO'})
    m.add_chunk('F21', 'PLUS-FACT', **{'ADDEND1': 'TWO', 'ADDEND2': 'ONE', 'SUM': 'THREE'})
    m.add_chunk('F22', 'PLUS-FACT', **{'ADDEND1': 'TWO', 'ADDEND2': 'TWO', 'SUM': 'FOUR'})
    m.add_chunk('F23', 'PLUS-FACT', **{'ADDEND1': 'TWO', 'ADDEND2': 'THREE', 'SUM': 'FIVE'})
    m.add_chunk('F24', 'PLUS-FACT', **{'ADDEND1': 'TWO', 'ADDEND2': 'FOUR', 'SUM': 'SIX'})
    m.add_chunk('F25', 'PLUS-FACT', **{'ADDEND1': 'TWO', 'ADDEND2': 'FIVE', 'SUM': 'SEVEN'})
    m.add_chunk('F30', 'PLUS-FACT', **{'ADDEND1': 'THREE', 'ADDEND2': 'ZERO', 'SUM': 'THREE'})
    m.add_chunk('F31', 'PLUS-FACT', **{'ADDEND1': 'THREE', 'ADDEND2': 'ONE', 'SUM': 'FOUR'})
    m.add_chunk('F32', 'PLUS-FACT', **{'ADDEND1': 'THREE', 'ADDEND2': 'TWO', 'SUM': 'FIVE'})
    m.add_chunk('F33', 'PLUS-FACT', **{'ADDEND1': 'THREE', 'ADDEND2': 'THREE', 'SUM': 'SIX'})
    m.add_chunk('F34', 'PLUS-FACT', **{'ADDEND1': 'THREE', 'ADDEND2': 'FOUR', 'SUM': 'SEVEN'})
    m.add_chunk('F35', 'PLUS-FACT', **{'ADDEND1': 'THREE', 'ADDEND2': 'FIVE', 'SUM': 'EIGHT'})
    m.add_chunk('F40', 'PLUS-FACT', **{'ADDEND1': 'FOUR', 'ADDEND2': 'ZERO', 'SUM': 'FOUR'})
    m.add_chunk('F41', 'PLUS-FACT', **{'ADDEND1': 'FOUR', 'ADDEND2': 'ONE', 'SUM': 'FIVE'})
    m.add_chunk('F42', 'PLUS-FACT', **{'ADDEND1': 'FOUR', 'ADDEND2': 'TWO', 'SUM': 'SIX'})
    m.add_chunk('F43', 'PLUS-FACT', **{'ADDEND1': 'FOUR', 'ADDEND2': 'THREE', 'SUM': 'SEVEN'})
    m.add_chunk('F44', 'PLUS-FACT', **{'ADDEND1': 'FOUR', 'ADDEND2': 'FOUR', 'SUM': 'EIGHT'})
    m.add_chunk('F45', 'PLUS-FACT', **{'ADDEND1': 'FOUR', 'ADDEND2': 'FIVE', 'SUM': 'NINE'})
    m.add_chunk('F50', 'PLUS-FACT', **{'ADDEND1': 'FIVE', 'ADDEND2': 'ZERO', 'SUM': 'FIVE'})
    m.add_chunk('F51', 'PLUS-FACT', **{'ADDEND1': 'FIVE', 'ADDEND2': 'ONE', 'SUM': 'SIX'})
    m.add_chunk('F52', 'PLUS-FACT', **{'ADDEND1': 'FIVE', 'ADDEND2': 'TWO', 'SUM': 'SEVEN'})
    m.add_chunk('F53', 'PLUS-FACT', **{'ADDEND1': 'FIVE', 'ADDEND2': 'THREE', 'SUM': 'EIGHT'})
    m.add_chunk('F54', 'PLUS-FACT', **{'ADDEND1': 'FIVE', 'ADDEND2': 'FOUR', 'SUM': 'NINE'})
    m.add_production(
        '''(P HEAR-SOUND =AURAL-LOCATION> ISA AUDIO-EVENT LOCATION EXTERNAL ?AURAL> STATE FREE ==> +AURAL> EVENT =AURAL-LOCATION)'''
    )
    m.add_production(
        '''(P ENCODE-DIGIT =AURAL> ISA SOUND CONTENT =VAL ?RETRIEVAL> STATE FREE ==> +RETRIEVAL> ISA NUMBER AURAL-REP =VAL)'''
    )
    m.add_production(
        '''(P HARVEST-ARG1 =RETRIEVAL> ?IMAGINAL> BUFFER EMPTY STATE FREE ==> +IMAGINAL> ISA PLUS-FACT ADDEND1 =RETRIEVAL)'''
    )
    m.add_production(
        '''(P HARVEST-ARG2 =RETRIEVAL> =IMAGINAL> ISA PLUS-FACT ADDEND2 NIL ?IMAGINAL> STATE FREE ==> *IMAGINAL> ADDEND2 =RETRIEVAL)'''
    )
    m.add_production(
        '''(P RETRIEVE-ANSWER =IMAGINAL> ISA PLUS-FACT ADDEND1 =VAL1 ADDEND2 =VAL2 SUM NIL ?RETRIEVAL> STATE FREE BUFFER EMPTY ==> =IMAGINAL> +RETRIEVAL> ISA PLUS-FACT ADDEND1 =VAL1 ADDEND2 =VAL2)'''
    )
    m.add_production(
        '''(P HARVEST-ANSWER =RETRIEVAL> ISA PLUS-FACT SUM =NUMBER =IMAGINAL> ISA PLUS-FACT ?IMAGINAL> STATE FREE ==> *IMAGINAL> SUM =NUMBER +RETRIEVAL> =NUMBER)'''
    )
    m.add_production(
        '''(P RESPOND =IMAGINAL> ISA PLUS-FACT SUM =RESULT =RETRIEVAL> ISA NUMBER VOCAL-REP =NAME ?VOCAL> STATE FREE ==> +VOCAL> CMD SPEAK STRING =NAME)'''
    )
    m.set_base_levels([('ZERO', 10), ('ONE', 10), ('TWO', 10), ('THREE', 10), ('FOUR', 10), ('FIVE', 10), ('SIX', 10), ('SEVEN', 10), ('EIGHT', 10), ('NINE', 10)])
    m.set_similarities([('ZERO', 'ONE', -0.1), ('ONE', 'TWO', -0.1), ('TWO', 'THREE', -0.1), ('THREE', 'FOUR', -0.1), ('FOUR', 'FIVE', -0.1), ('ZERO', 'TWO', -0.2), ('ONE', 'THREE', -0.2), ('TWO', 'FOUR', -0.2), ('THREE', 'FIVE', -0.2), ('ZERO', 'THREE', -0.3), ('ONE', 'FOUR', -0.3), ('TWO', 'FIVE', -0.3), ('ZERO', 'FOUR', -0.4), ('ONE', 'FIVE', -0.4), ('ZERO', 'FIVE', -0.5)])
    m.set_base_levels([('F00', 0.65), ('F01', 0.65), ('F02', 0.65), ('F03', 0.65), ('F04', 0.65), ('F10', 0.65), ('F11', 0.65), ('F12', 0.65), ('F13', 0.65), ('F20', 0.65), ('F21', 0.65), ('F22', 0.65), ('F30', 0.65), ('F31', 0.65), ('F40', 0.65)])
    return m
