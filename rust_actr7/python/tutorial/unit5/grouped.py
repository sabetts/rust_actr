"""Python port of ACT-R:tutorial;unit5;grouped-model.lisp plus the grouped task."""

from __future__ import annotations

import json
import sys

import rust_actr.actr as actr

LOGICAL_PATH = 'ACT-R:tutorial;unit5;grouped-model.lisp'
MODEL_NAME = 'GROUPED'
DEFAULT_FUNCTION = 'recall'
DEFAULT_ARGS = []


def build(seed=(100, 0)) -> actr.Model:
    m = actr.Model('GROUPED', seed=seed)
    m.sgp(':RT', -0.5)
    m.sgp(':ANS', 0.15)
    m.sgp(':ESC', True)
    m.sgp(':MP', 1.0)
    m.sgp(':V', True)
    m.sgp(':ACT', None)
    m.sgp(':DECLARATIVE-NUM-FINSTS', 15)
    m.sgp(':DECLARATIVE-FINST-SPAN', 20)
    m.sgp(':TRACE-DETAIL', 'LOW')
    m.sgp(':SEED', [77093543977, 39813])
    m.chunk_type('RECALL-LIST', 'GROUP', 'ELEMENT', 'LIST', 'GROUP-POSITION')
    m.chunk_type('GROUP', 'ID', 'PARENT', 'POSITION')
    m.chunk_type('ITEM', 'NAME', 'GROUP', 'POSITION')
    m.add_chunk('LIST', 'CHUNK', **{})
    m.add_chunk('FIRST', 'CHUNK', **{})
    m.add_chunk('SECOND', 'CHUNK', **{})
    m.add_chunk('THIRD', 'CHUNK', **{})
    m.add_chunk('FOURTH', 'CHUNK', **{})
    m.add_chunk('GROUP1', 'GROUP', **{'PARENT': 'LIST', 'POSITION': 'FIRST', 'ID': 'GROUP1'})
    m.add_chunk('GROUP2', 'GROUP', **{'PARENT': 'LIST', 'POSITION': 'SECOND', 'ID': 'GROUP2'})
    m.add_chunk('GROUP3', 'GROUP', **{'PARENT': 'LIST', 'POSITION': 'THIRD', 'ID': 'GROUP3'})
    m.add_chunk('ITEM1', 'ITEM', **{'NAME': actr.String('1'), 'GROUP': 'GROUP1', 'POSITION': 'FIRST'})
    m.add_chunk('ITEM2', 'ITEM', **{'NAME': actr.String('2'), 'GROUP': 'GROUP1', 'POSITION': 'SECOND'})
    m.add_chunk('ITEM3', 'ITEM', **{'NAME': actr.String('3'), 'GROUP': 'GROUP1', 'POSITION': 'THIRD'})
    m.add_chunk('ITEM4', 'ITEM', **{'NAME': actr.String('4'), 'GROUP': 'GROUP2', 'POSITION': 'FIRST'})
    m.add_chunk('ITEM5', 'ITEM', **{'NAME': actr.String('5'), 'GROUP': 'GROUP2', 'POSITION': 'SECOND'})
    m.add_chunk('ITEM6', 'ITEM', **{'NAME': actr.String('6'), 'GROUP': 'GROUP2', 'POSITION': 'THIRD'})
    m.add_chunk('ITEM7', 'ITEM', **{'NAME': actr.String('7'), 'GROUP': 'GROUP3', 'POSITION': 'FIRST'})
    m.add_chunk('ITEM8', 'ITEM', **{'NAME': actr.String('8'), 'GROUP': 'GROUP3', 'POSITION': 'SECOND'})
    m.add_chunk('ITEM9', 'ITEM', **{'NAME': actr.String('9'), 'GROUP': 'GROUP3', 'POSITION': 'THIRD'})
    m.add_production(
        '''(P RECALL-FIRST-GROUP =GOAL> ISA RECALL-LIST LIST =LIST ?RETRIEVAL> BUFFER EMPTY STATE FREE ==> =GOAL> GROUP-POSITION FIRST +RETRIEVAL> ISA GROUP PARENT =LIST POSITION FIRST)'''
    )
    m.add_production(
        '''(P START-RECALL-OF-GROUP =GOAL> ISA RECALL-LIST LIST =LIST =RETRIEVAL> ISA GROUP ID =GROUP ==> =GOAL> GROUP =GROUP ELEMENT FIRST +RETRIEVAL> ISA ITEM GROUP =GROUP POSITION FIRST :RECENTLY-RETRIEVED NIL)'''
    )
    m.add_production(
        '''(P HARVEST-FIRST-ITEM =GOAL> ISA RECALL-LIST ELEMENT FIRST GROUP =GROUP =RETRIEVAL> ISA ITEM NAME =NAME ==> =GOAL> ELEMENT SECOND +RETRIEVAL> ISA ITEM GROUP =GROUP POSITION SECOND :RECENTLY-RETRIEVED NIL !EVAL! ("grouped-response" =NAME))'''
    )
    m.add_production(
        '''(P HARVEST-SECOND-ITEM =GOAL> ISA RECALL-LIST ELEMENT SECOND GROUP =GROUP =RETRIEVAL> ISA ITEM NAME =NAME ==> =GOAL> ELEMENT THIRD +RETRIEVAL> ISA ITEM GROUP =GROUP POSITION THIRD :RECENTLY-RETRIEVED NIL !EVAL! ("grouped-response" =NAME))'''
    )
    m.add_production(
        '''(P HARVEST-THIRD-ITEM =GOAL> ISA RECALL-LIST ELEMENT THIRD GROUP =GROUP =RETRIEVAL> ISA ITEM NAME =NAME ==> =GOAL> ELEMENT FOURTH +RETRIEVAL> ISA ITEM GROUP =GROUP POSITION FOURTH :RECENTLY-RETRIEVED NIL !EVAL! ("grouped-response" =NAME))'''
    )
    m.add_production(
        '''(P SECOND-GROUP =GOAL> ISA RECALL-LIST GROUP-POSITION FIRST LIST =LIST ?RETRIEVAL> BUFFER FAILURE ==> =GOAL> GROUP-POSITION SECOND +RETRIEVAL> ISA GROUP PARENT =LIST POSITION SECOND)'''
    )
    m.add_production(
        '''(P THIRD-GROUP =GOAL> ISA RECALL-LIST GROUP-POSITION SECOND LIST =LIST ?RETRIEVAL> BUFFER FAILURE ==> =GOAL> GROUP-POSITION THIRD +RETRIEVAL> ISA GROUP PARENT =LIST POSITION THIRD)'''
    )
    m.goal_focus(isa='RECALL-LIST', **{'LIST': 'LIST'})
    m.set_similarities([('FIRST', 'SECOND', -0.5), ('SECOND', 'THIRD', -0.5)])
    return m


actr.clear_state()

# ACT-R tutorial unit 5 grouped task.
# This is a simple example task to show partial matching.
# It simply runs the model and records values that the
# model provides and returns the list of provided values
# in the order provided after the run.

# Import the actr module for tutorial tasks


# Load the corresponding model for the task.

MODEL = actr.use(build())

# Create a variable to hold the responses

response = []

# The recall function creates a command for
# the record_response function, clears the response
# list, runs the model, and returns the response list.

def recall ():

    actr.add_command("grouped-response",record_response,"Response recording function for the tutorial grouped model.")
    global response
    response = []
    actr.reset()
    actr.run(20)
    actr.remove_command("grouped-response")
    return response


# Store a value provided by the model on the response list

def record_response (item):

    global response
    response.append(item)


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
