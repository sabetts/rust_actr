"""Python port of ACT-R:tutorial;unit7;utility-learning-issues-model.lisp plus the ul_issues task."""

from __future__ import annotations

import json
import sys

import rust_actr.actr as actr

DEFAULT_FUNCTION = None
DEFAULT_ARGS = []


def build(seed=(100, 0)) -> actr.Model:
    m = actr.Model('utility-learning-test', seed=seed)

    m.sgp(':seed', [100, 2])
    m.sgp(':esc', True)
    m.sgp(':ul', True)
    m.sgp(':egs', 0.5)

    m.chunk_type('choose', 'choice')
    m.chunk_type('response', 'answer')

    m.define_chunks('a')
    m.define_chunks('b')

    m.declare_buffer_usage('imaginal', 'response', 'answer')

    m.add_production(
        '''
        (p choose-a
           =goal>
             isa choose
             choice nil
         ==>
           =goal>
             choice a)
        '''
    )

    m.add_production(
        '''
        (p choose-b
           =goal>
             isa choose
             choice nil
         ==>
           =goal>
             choice b)
        '''
    )

    m.add_production(
        '''
        (p response-matches
           =goal>
             isa choose
             choice =choice
           =imaginal>
             isa response
             answer =choice
         ==>
           -goal>)
        '''
    )

    m.add_production(
        '''
        (p response-doesnt-match
           =goal>
             isa choose
             choice =choice
           =imaginal>
             isa response
             - answer =choice
             - answer nil
         ==>
           -goal>)
        '''
    )

    m.add_production(
        '''
        (p unknown-response
           =goal>
             isa choose
             choice =choice
           =imaginal>
             isa response
             answer nil
         ==>
           -goal>)
        '''
    )
    m.spp('response-matches', **{'reward': 4})
    m.spp('response-doesnt-match', **{'reward': 0})

    m.form('(schedule-event 0 "utility-learning-issues-choose" :output (quote medium))')
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
