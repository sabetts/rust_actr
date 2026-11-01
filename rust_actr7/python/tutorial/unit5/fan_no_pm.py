"""Python port of ACT-R:tutorial;unit5;fan-no-pm-model.lisp plus the fan_no_pm task."""

from __future__ import annotations

import json
import sys

import rust_actr.actr as actr

LOGICAL_PATH = 'ACT-R:tutorial;unit5;fan-no-pm-model.lisp'
MODEL_NAME = 'FAN'
DEFAULT_FUNCTION = 'experiment'
DEFAULT_ARGS = []


def build(seed=(100, 0)) -> actr.Model:
    m = actr.Model('FAN', seed=seed)
    m.sgp(':V', True)
    m.sgp(':ACT', None)
    m.sgp(':ESC', True)
    m.sgp(':LF', 0.63)
    m.sgp(':MAS', 1.6)
    m.sgp(':GA', 1.0)
    m.sgp(':STYLE-WARNINGS', None)
    m.chunk_type('COMPREHEND-SENTENCE', 'RELATION', 'ARG1', 'ARG2')
    m.chunk_type('MEANING', 'WORD')
    m.add_chunk('P1', 'COMPREHEND-SENTENCE', **{'RELATION': 'IN', 'ARG1': 'HIPPIE', 'ARG2': 'PARK'})
    m.add_chunk('P2', 'COMPREHEND-SENTENCE', **{'RELATION': 'IN', 'ARG1': 'HIPPIE', 'ARG2': 'CHURCH'})
    m.add_chunk('P3', 'COMPREHEND-SENTENCE', **{'RELATION': 'IN', 'ARG1': 'HIPPIE', 'ARG2': 'BANK'})
    m.add_chunk('P4', 'COMPREHEND-SENTENCE', **{'RELATION': 'IN', 'ARG1': 'CAPTAIN', 'ARG2': 'PARK'})
    m.add_chunk('P5', 'COMPREHEND-SENTENCE', **{'RELATION': 'IN', 'ARG1': 'CAPTAIN', 'ARG2': 'CAVE'})
    m.add_chunk('P6', 'COMPREHEND-SENTENCE', **{'RELATION': 'IN', 'ARG1': 'DEBUTANTE', 'ARG2': 'BANK'})
    m.add_chunk('P7', 'COMPREHEND-SENTENCE', **{'RELATION': 'IN', 'ARG1': 'FIREMAN', 'ARG2': 'PARK'})
    m.add_chunk('P8', 'COMPREHEND-SENTENCE', **{'RELATION': 'IN', 'ARG1': 'GIANT', 'ARG2': 'BEACH'})
    m.add_chunk('P9', 'COMPREHEND-SENTENCE', **{'RELATION': 'IN', 'ARG1': 'GIANT', 'ARG2': 'CASTLE'})
    m.add_chunk('P10', 'COMPREHEND-SENTENCE', **{'RELATION': 'IN', 'ARG1': 'GIANT', 'ARG2': 'DUNGEON'})
    m.add_chunk('P11', 'COMPREHEND-SENTENCE', **{'RELATION': 'IN', 'ARG1': 'EARL', 'ARG2': 'CASTLE'})
    m.add_chunk('P12', 'COMPREHEND-SENTENCE', **{'RELATION': 'IN', 'ARG1': 'EARL', 'ARG2': 'FOREST'})
    m.add_chunk('P13', 'COMPREHEND-SENTENCE', **{'RELATION': 'IN', 'ARG1': 'LAWYER', 'ARG2': 'STORE'})
    m.add_chunk('GUARD', 'MEANING', **{'WORD': actr.String('guard')})
    m.add_chunk('BEACH', 'MEANING', **{'WORD': actr.String('beach')})
    m.add_chunk('CASTLE', 'MEANING', **{'WORD': actr.String('castle')})
    m.add_chunk('DUNGEON', 'MEANING', **{'WORD': actr.String('dungeon')})
    m.add_chunk('EARL', 'MEANING', **{'WORD': actr.String('earl')})
    m.add_chunk('FOREST', 'MEANING', **{'WORD': actr.String('forest')})
    m.add_chunk('GIANT', 'MEANING', **{'WORD': actr.String('giant')})
    m.add_chunk('HIPPIE', 'MEANING', **{'WORD': actr.String('hippie')})
    m.add_chunk('PARK', 'MEANING', **{'WORD': actr.String('park')})
    m.add_chunk('CHURCH', 'MEANING', **{'WORD': actr.String('church')})
    m.add_chunk('CAPTAIN', 'MEANING', **{'WORD': actr.String('captain')})
    m.add_chunk('CAVE', 'MEANING', **{'WORD': actr.String('cave')})
    m.add_chunk('DEBUTANTE', 'MEANING', **{'WORD': actr.String('debutante')})
    m.add_chunk('BANK', 'MEANING', **{'WORD': actr.String('bank')})
    m.add_chunk('FIREMAN', 'MEANING', **{'WORD': actr.String('fireman')})
    m.add_chunk('LAWYER', 'MEANING', **{'WORD': actr.String('lawyer')})
    m.add_chunk('STORE', 'MEANING', **{'WORD': actr.String('store')})
    m.add_chunk('IN', 'MEANING', **{'WORD': actr.String('in')})
    m.set_base_levels([('GUARD', 10), ('BEACH', 10), ('CASTLE', 10), ('DUNGEON', 10), ('EARL', 10), ('FOREST', 10), ('HIPPIE', 10), ('PARK', 10), ('CHURCH', 10), ('BANK', 10), ('CAPTAIN', 10), ('CAVE', 10), ('GIANT', 10), ('DEBUTANTE', 10), ('FIREMAN', 10), ('LAWYER', 10), ('STORE', 10), ('IN', 10)])
    m.chunk_type('SENTENCE-GOAL', 'ARG1', 'ARG2', 'STATE')
    m.define_chunks(actr.Chunk('TEST', 'CHUNK', {}))
    m.define_chunks(actr.Chunk('HARVEST-PERSON', 'CHUNK', {}))
    m.define_chunks(actr.Chunk('HARVEST-LOCATION', 'CHUNK', {}))
    m.define_chunks(actr.Chunk('GET-RETRIEVAL', 'CHUNK', {}))
    m.goal_focus(isa='SENTENCE-GOAL', **{'STATE': 'TEST'})
    m.add_production(
        '''(P START =GOAL> ISA SENTENCE-GOAL ARG1 =PERSON STATE TEST ==> =GOAL> STATE HARVEST-PERSON +RETRIEVAL> ISA MEANING WORD =PERSON)'''
    )
    m.add_production(
        '''(P HARVEST-PERSON =GOAL> ISA SENTENCE-GOAL ARG2 =LOCATION STATE HARVEST-PERSON =RETRIEVAL> ==> =GOAL> ARG1 =RETRIEVAL STATE HARVEST-LOCATION +RETRIEVAL> ISA MEANING WORD =LOCATION)'''
    )
    m.add_production(
        '''(P HARVEST-LOCATION =GOAL> ISA SENTENCE-GOAL STATE HARVEST-LOCATION =RETRIEVAL> ISA MEANING ==> =GOAL> ARG2 =RETRIEVAL STATE GET-RETRIEVAL)'''
    )
    m.add_production(
        '''(P RETRIEVE-FROM-PERSON =GOAL> ISA SENTENCE-GOAL ARG1 =PERSON STATE GET-RETRIEVAL ==> =GOAL> STATE NIL +RETRIEVAL> ISA COMPREHEND-SENTENCE ARG1 =PERSON)'''
    )
    m.add_production(
        '''(P RETRIEVE-FROM-LOCATION =GOAL> ISA SENTENCE-GOAL ARG2 =LOCATION STATE GET-RETRIEVAL ==> =GOAL> STATE NIL +RETRIEVAL> ISA COMPREHEND-SENTENCE ARG2 =LOCATION)'''
    )
    m.add_production(
        '''(P RESPOND-YES =GOAL> ISA SENTENCE-GOAL ARG1 =PERSON ARG2 =LOCATION STATE NIL =RETRIEVAL> ISA COMPREHEND-SENTENCE ARG1 =PERSON ARG2 =LOCATION ==> =GOAL> STATE "k")'''
    )
    m.add_production(
        '''(P MISMATCH-PERSON-NO =GOAL> ISA SENTENCE-GOAL ARG1 =PERSON ARG2 =LOCATION STATE NIL =RETRIEVAL> ISA COMPREHEND-SENTENCE - ARG1 =PERSON ==> =GOAL> STATE "d")'''
    )
    m.add_production(
        '''(P MISMATCH-LOCATION-NO =GOAL> ISA SENTENCE-GOAL ARG1 =PERSON ARG2 =LOCATION STATE NIL =RETRIEVAL> ISA COMPREHEND-SENTENCE - ARG2 =LOCATION ==> =GOAL> STATE "d")'''
    )
    m.spp('MISMATCH-LOCATION-NO', **{'AT': 0.21})
    m.spp('MISMATCH-PERSON-NO', **{'AT': 0.21})
    m.spp('RESPOND-YES', **{'AT': 0.21})
    m.spp('START', **{'AT': 0.25})
    m.spp('HARVEST-PERSON', **{'AT': 0.285})
    return m


actr.clear_state()

# ACT-R tutorial unit 5 fan task.
# This experiment presents a model with a person-location pair
# of items and the model must respond whether that pair of items
# was part of the study set that it has recorded in memory.
# The task and data to which the model is fit are in the paper:
#
# Anderson, J. R. (1974). Retrieval of propositional information from
# long-term memory. Cognitive Psychology, 5, 451 - 474.
#
# The results are reported are the time to respond to the probe
# based on the 'fan' of the items presented (how many places a person
# is in or how many people are in the place) and whether the probe
# is or isn't in the test set.
#
# This version of the task does not use the perceptual or motor
# modules of ACT-R and instead places the probes directly into
# slots of the goal buffer and reads the model's response from
# a slot of the goal buffer when done.

# Import the actr module for tutorial tasks


# Load the corresponding model for the task.

MODEL = actr.use(build())

# Create a variable with the original experiment data.

person_location_data = [1.11, 1.17, 1.22,
                        1.17, 1.20, 1.22,
                        1.15, 1.23, 1.36,
                        1.20, 1.22, 1.26,
                        1.25, 1.36, 1.29,
                        1.26, 1.47, 1.47]

# The sentence function takes 4 parameters.
# The first two are the strings with a string of the person and location
# to present e.g. "'hippie'".  The third is True or False to indicate whether
# this was or wasn't in the study set, and the last is
# either the string 'person' or 'location' to indicate which
# of the productions the model should use for retrieval.
#
# It presents the probe items given directly to the model
# through the goal buffer, runs the model, and
# returns a tuple indicating how many seconds the model
# ran and True or False to indicate if the response was correct.

def sentence (person, location, target, term):

    actr.reset()

    # disable the production that isn't being used for retrieval

    if term == 'person':
        actr.pdisable("retrieve-from-location")
    else:
        actr.pdisable("retrieve-from-person")

    # modify the chunk in the goal buffer to set the
    # arg1 and arg2 slots to the probe items

    actr.mod_focus("arg1",person,"arg2",location)

    # run the model recording the time spent running
    # and get the value from the state slot of the goal buffer representing the
    # model's response to the task

    response_time = actr.run(30)[0]
    response = actr.buffer_slot_value("goal","state")

    if target:
        if response.lower() == "'k'".lower():
            return (response_time ,True)
        else:
            return (response_time ,False)
    else:
        if response.lower() == "'d'".lower():
            return (response_time ,True)
        else:
            return (response_time ,False)
   
# do_person_location requires one parameter which is either
# the string 'person' or 'location' to indicate which of the 
# productions the model should use for retrieval.
# It runs one trial of each fan condition and returns a list
# of the results.
    
def do_person_location(term):

    data = []

    for person,location,target in [("'lawyer'", "'store'", True),
                                   ("'captain'", "'cave'", True),
                                   ("'hippie'", "'church'", True),
                                   ("'debutante'", "'bank'", True),
                                   ("'earl'", "'castle'", True),
                                   ("'hippie'", "'bank'", True),
                                   ("'fireman'", "'park'", True),
                                   ("'captain'", "'park'", True),
                                   ("'hippie'", "'park'", True),
                                   ("'fireman'", "'store'", False),
                                   ("'captain'", "'store'", False),
                                   ("'giant'", "'store'", False),
                                   ("'fireman'", "'bank'", False),
                                   ("'captain'", "'bank'", False),
                                   ("'giant'", "'bank'", False),
                                   ("'lawyer'", "'park'", False),
                                   ("'earl'", "'park'", False),
                                   ("'giant'", "'park'", False)]:

        data.append(sentence(person,location,target,term))

    return data

# experiment runs the model through one trial of 
# each condition using each of the retrieval productions
# and averages the results then displays the results.

def experiment():

    output_person_location(list(map(lambda x,y:((x[0]+y[0])/2,(x[1] and y[1])),
                                    do_person_location('person'),
                                    do_person_location('location'))))


def output_person_location(data):

    rts = list(map(lambda x: x[0],data))

    actr.correlation(rts,person_location_data)
    actr.mean_deviation(rts,person_location_data)

    print("\nTARGETS:\n                         Person fan")
    print("  Location      1             2             3")
    print("    fan")
    
    for i in range(3):
        print("     %d      " % (i+1),end="")
        for j in range(3):
            print("%6.3f (%-5s)" % (data[j + (i * 3)]),end="")
        print()

    print()
    print("FOILS:")
    for i in range(3):
        print("     %d      " % (i+1),end="")
        for j in range(3):
            print("%6.3f (%-5s)" % (data[j + ((i + 3) * 3)]),end="")
        print()


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
