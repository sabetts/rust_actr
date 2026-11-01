"""Python port of ACT-R:tutorial;unit7;utility-learning-issues-model.lisp plus the ul_issues task."""

from __future__ import annotations

import json
import sys

import rust_actr.actr as actr

LOGICAL_PATH = 'ACT-R:tutorial;unit7;utility-learning-issues-model.lisp'
MODEL_NAME = 'UTILITY-LEARNING-TEST'
DEFAULT_FUNCTION = None
DEFAULT_ARGS = []


def build(seed=(100, 0)) -> actr.Model:
    m = actr.Model('UTILITY-LEARNING-TEST', seed=seed)
    m.sgp(':SEED', [100, 2])
    m.sgp(':ESC', True)
    m.sgp(':UL', True)
    m.sgp(':EGS', 0.5)
    m.chunk_type('CHOOSE', 'CHOICE')
    m.chunk_type('RESPONSE', 'ANSWER')
    m.define_chunks('A')
    m.define_chunks('B')
    m.declare_buffer_usage('IMAGINAL', 'RESPONSE', 'ANSWER')
    m.add_production(
        '''(P CHOOSE-A =GOAL> ISA CHOOSE CHOICE NIL ==> =GOAL> CHOICE A)'''
    )
    m.add_production(
        '''(P CHOOSE-B =GOAL> ISA CHOOSE CHOICE NIL ==> =GOAL> CHOICE B)'''
    )
    m.add_production(
        '''(P RESPONSE-MATCHES =GOAL> ISA CHOOSE CHOICE =CHOICE =IMAGINAL> ISA RESPONSE ANSWER =CHOICE ==> -GOAL>)'''
    )
    m.add_production(
        '''(P RESPONSE-DOESNT-MATCH =GOAL> ISA CHOOSE CHOICE =CHOICE =IMAGINAL> ISA RESPONSE - ANSWER =CHOICE - ANSWER NIL ==> -GOAL>)'''
    )
    m.add_production(
        '''(P UNKNOWN-RESPONSE =GOAL> ISA CHOOSE CHOICE =CHOICE =IMAGINAL> ISA RESPONSE ANSWER NIL ==> -GOAL>)'''
    )
    m.spp('RESPONSE-MATCHES', **{'REWARD': 4})
    m.spp('RESPONSE-DOESNT-MATCH', **{'REWARD': 0})
    m.form('(SCHEDULE-EVENT 0 "utility-learning-issues-choose" :OUTPUT (QUOTE MEDIUM))')
    return m


actr.clear_state()

# ACT-R tutorial unit7 task for investigating utility
# learning modeling issues.


def present_choose():

    actr.goal_focus(['isa','choose'])

    if actr.random(1.0) < .6:
        answer = 'a' 
    elif actr.random(1.0) > .5:
        answer = 'b'
    else:
        answer = None

    actr.schedule_event_relative(5,'utility-learning-issues-show-result',params=[answer],output='medium')


def show_result (choice):
    actr.set_buffer_chunk('imaginal',['answer',choice])
    actr.schedule_event_relative(2,'utility-learning-issues-choose',output='medium')


actr.add_command('utility-learning-issues-choose',present_choose,"Function to change model's goal for utility learning issues model")
actr.add_command('utility-learning-issues-show-result',show_result,"Function to set the model's imaginal buffer for utility learning issues model")


def finished ():
    actr.remove_command('utility-learning-issues-choose')
    actr.remove_command('utility-learning-issues-show-result')


MODEL = actr.use(build())


def _value(text):
    try:
        return json.loads(text)
    except json.JSONDecodeError:
        return text


def main(argv=None):
    argv = list(sys.argv[1:] if argv is None else argv)
    if argv:
        name = argv[0]
        if name not in globals():
            public = sorted(
                key
                for key, value in globals().items()
                if callable(value) and not key.startswith('_')
            )
            raise SystemExit(
                f'unknown function {name!r}; available: {public}'
            )
        function = globals()[name]
        arguments = [_value(arg) for arg in argv[1:]]
    elif DEFAULT_FUNCTION is not None:
        function = globals()[DEFAULT_FUNCTION]
        arguments = list(DEFAULT_ARGS)
    else:
        print(MODEL.run(10.0))
        return None
    result = function(*arguments)
    if result is not None:
        print(result)
    return result


if __name__ == "__main__":
    main()
