"""Python port of ACT-R:tutorial;unit5;grouped-model.lisp plus the grouped task."""

from __future__ import annotations

import json
import sys

import rust_actr.actr as actr

DEFAULT_FUNCTION = 'recall'
DEFAULT_ARGS = []


def build(seed=(100, 0)) -> actr.Model:
    m = actr.Model('grouped', seed=seed)

    m.sgp(':rt', -0.5)
    m.sgp(':ans', 0.15)
    m.sgp(':esc', True)
    m.sgp(':mp', 1.0)
    m.sgp(':v', True)
    m.sgp(':act', None)
    m.sgp(':declarative-num-finsts', 15)
    m.sgp(':declarative-finst-span', 20)
    m.sgp(':trace-detail', 'low')
    m.sgp(':seed', [77093543977, 39813])

    m.chunk_type('recall-list', 'group', 'element', 'list', 'group-position')
    m.chunk_type('group', 'id', 'parent', 'position')
    m.chunk_type('item', 'name', 'group', 'position')

    m.add_chunk('list', 'chunk')
    m.add_chunk('first', 'chunk')
    m.add_chunk('second', 'chunk')
    m.add_chunk('third', 'chunk')
    m.add_chunk('fourth', 'chunk')
    m.add_chunk('group1', 'group', parent='list', position='first', id='group1')
    m.add_chunk('group2', 'group', parent='list', position='second', id='group2')
    m.add_chunk('group3', 'group', parent='list', position='third', id='group3')
    m.add_dm(actr.Chunk('item1', 'item', {'name': actr.String('1'), 'group': 'group1', 'position': 'first'}))
    m.add_dm(actr.Chunk('item2', 'item', {'name': actr.String('2'), 'group': 'group1', 'position': 'second'}))
    m.add_dm(actr.Chunk('item3', 'item', {'name': actr.String('3'), 'group': 'group1', 'position': 'third'}))
    m.add_dm(actr.Chunk('item4', 'item', {'name': actr.String('4'), 'group': 'group2', 'position': 'first'}))
    m.add_dm(actr.Chunk('item5', 'item', {'name': actr.String('5'), 'group': 'group2', 'position': 'second'}))
    m.add_dm(actr.Chunk('item6', 'item', {'name': actr.String('6'), 'group': 'group2', 'position': 'third'}))
    m.add_dm(actr.Chunk('item7', 'item', {'name': actr.String('7'), 'group': 'group3', 'position': 'first'}))
    m.add_dm(actr.Chunk('item8', 'item', {'name': actr.String('8'), 'group': 'group3', 'position': 'second'}))
    m.add_dm(actr.Chunk('item9', 'item', {'name': actr.String('9'), 'group': 'group3', 'position': 'third'}))

    m.add_production(
        '''
        (p recall-first-group
           =goal>
              isa      recall-list
              list     =list
           ?retrieval>
              buffer    empty
              state     free
          ==>
           =goal>
              group-position first
           +retrieval>
              isa      group
              parent   =list
              position first)
        '''
    )

    m.add_production(
        '''
        (p start-recall-of-group
           =goal>
              isa      recall-list
              list     =list
           =retrieval>
              isa      group
              id       =group
          ==>
           =goal>
              group    =group
              element  first
           +retrieval>
              isa      item
              group    =group
              position first
              :recently-retrieved nil)
        '''
    )

    m.add_production(
        '''
        (p harvest-first-item
           =goal>
              isa      recall-list
              element  first
              group    =group
           =retrieval>
              isa      item
              name     =name
          ==>
           =goal>
              element  second
           +retrieval>
              isa      item
              group    =group
              position second
              :recently-retrieved nil
           !eval! ("grouped-response" =name))
        '''
    )

    m.add_production(
        '''
        (p harvest-second-item
           =goal>
              isa      recall-list
              element  second
              group    =group
           =retrieval>
              isa      item
              name     =name
          ==>
           =goal>
              element  third
           +retrieval>
              isa      item
              group    =group
              position third
              :recently-retrieved nil
           !eval! ("grouped-response" =name))
        '''
    )

    m.add_production(
        '''
        (p harvest-third-item
           =goal>
              isa      recall-list
              element  third
              group    =group
           =retrieval>
              isa      item
              name     =name
          ==>
           =goal>
              element  fourth
           +retrieval>
              isa      item
              group    =group
              position fourth
              :recently-retrieved nil
           !eval! ("grouped-response" =name))
        '''
    )

    m.add_production(
        '''
        (p second-group
           =goal>
              isa      recall-list
              group-position first
              list     =list
           ?retrieval>
              buffer   failure
          ==>
           =goal>
              group-position second
           +retrieval>
              isa      group
              parent   =list
              position second)
        '''
    )

    m.add_production(
        '''
        (p third-group
           =goal>
              isa      recall-list
              group-position second
              list     =list
           ?retrieval>
              buffer   failure
          ==>
           =goal>
              group-position third
           +retrieval>
              isa      group
              parent   =list
              position third)
        '''
    )

    m.goal_focus(isa='recall-list', list='list')

    m.set_similarities([('first', 'second', -0.5), ('second', 'third', -0.5)])
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
