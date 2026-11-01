"""Python port of ACT-R:tutorial;unit5;fan-no-pm-model.lisp plus the fan_no_pm task."""

from __future__ import annotations

import json
import sys

import rust_actr.actr as actr

DEFAULT_FUNCTION = 'experiment'
DEFAULT_ARGS = []


def build(seed=(100, 0)) -> actr.Model:
    m = actr.Model('fan', seed=seed)

    m.sgp('v', True)
    m.sgp('act', None)
    m.sgp('esc', True)
    m.sgp('lf', 0.63)
    m.sgp('mas', 1.6)
    m.sgp('ga', 1.0)
    m.sgp('style_warnings', None)

    m.chunk_type('comprehend-sentence', 'relation', 'arg1', 'arg2')
    m.chunk_type('meaning', 'word')

    m.add_chunk('p1', 'comprehend-sentence', relation='in', arg1='hippie', arg2='park')
    m.add_chunk('p2', 'comprehend-sentence', relation='in', arg1='hippie', arg2='church')
    m.add_chunk('p3', 'comprehend-sentence', relation='in', arg1='hippie', arg2='bank')
    m.add_chunk('p4', 'comprehend-sentence', relation='in', arg1='captain', arg2='park')
    m.add_chunk('p5', 'comprehend-sentence', relation='in', arg1='captain', arg2='cave')
    m.add_chunk('p6', 'comprehend-sentence', relation='in', arg1='debutante', arg2='bank')
    m.add_chunk('p7', 'comprehend-sentence', relation='in', arg1='fireman', arg2='park')
    m.add_chunk('p8', 'comprehend-sentence', relation='in', arg1='giant', arg2='beach')
    m.add_chunk('p9', 'comprehend-sentence', relation='in', arg1='giant', arg2='castle')
    m.add_chunk('p10', 'comprehend-sentence', relation='in', arg1='giant', arg2='dungeon')
    m.add_chunk('p11', 'comprehend-sentence', relation='in', arg1='earl', arg2='castle')
    m.add_chunk('p12', 'comprehend-sentence', relation='in', arg1='earl', arg2='forest')
    m.add_chunk('p13', 'comprehend-sentence', relation='in', arg1='lawyer', arg2='store')
    m.add_chunk('guard', 'meaning', word=actr.String('guard'))
    m.add_chunk('beach', 'meaning', word=actr.String('beach'))
    m.add_chunk('castle', 'meaning', word=actr.String('castle'))
    m.add_chunk('dungeon', 'meaning', word=actr.String('dungeon'))
    m.add_chunk('earl', 'meaning', word=actr.String('earl'))
    m.add_chunk('forest', 'meaning', word=actr.String('forest'))
    m.add_chunk('giant', 'meaning', word=actr.String('giant'))
    m.add_chunk('hippie', 'meaning', word=actr.String('hippie'))
    m.add_chunk('park', 'meaning', word=actr.String('park'))
    m.add_chunk('church', 'meaning', word=actr.String('church'))
    m.add_chunk('captain', 'meaning', word=actr.String('captain'))
    m.add_chunk('cave', 'meaning', word=actr.String('cave'))
    m.add_chunk('debutante', 'meaning', word=actr.String('debutante'))
    m.add_chunk('bank', 'meaning', word=actr.String('bank'))
    m.add_chunk('fireman', 'meaning', word=actr.String('fireman'))
    m.add_chunk('lawyer', 'meaning', word=actr.String('lawyer'))
    m.add_chunk('store', 'meaning', word=actr.String('store'))
    m.add_chunk('in', 'meaning', word=actr.String('in'))

    m.set_base_levels([('guard', 10), ('beach', 10), ('castle', 10), ('dungeon', 10), ('earl', 10), ('forest', 10), ('hippie', 10), ('park', 10), ('church', 10), ('bank', 10), ('captain', 10), ('cave', 10), ('giant', 10), ('debutante', 10), ('fireman', 10), ('lawyer', 10), ('store', 10), ('in', 10)])

    m.chunk_type('sentence-goal', 'arg1', 'arg2', 'state')

    m.define_chunks(actr.Chunk('test', 'chunk', {}))
    m.define_chunks(actr.Chunk('harvest-person', 'chunk', {}))
    m.define_chunks(actr.Chunk('harvest-location', 'chunk', {}))
    m.define_chunks(actr.Chunk('get-retrieval', 'chunk', {}))

    m.goal_focus(isa='sentence-goal', state='test')

    m.add_production(
        '''
        (p start
           =goal>
              isa         sentence-goal
              arg1        =person
              state       test
           ==>
           =goal>
              state       harvest-person
           +retrieval>
              isa         meaning
              word        =person)
        '''
    )

    m.add_production(
        '''
        (p harvest-person
           =goal>
              isa         sentence-goal
              arg2        =location
              state       harvest-person
           =retrieval>
           ==>
           =goal>
              arg1        =retrieval
              state       harvest-location
           +retrieval>
              isa         meaning
              word        =location)
        '''
    )

    m.add_production(
        '''
        (p harvest-location
           =goal>
              isa         sentence-goal
              state       harvest-location
           =retrieval>
              isa         meaning
           ==>
           =goal>
              arg2        =retrieval
              state       get-retrieval)
        '''
    )

    m.add_production(
        '''
        (p retrieve-from-person
           =goal>
              isa         sentence-goal
              arg1        =person
              state       get-retrieval
           ==>
           =goal>
              state       nil
           +retrieval>
              isa         comprehend-sentence
              arg1        =person)
        '''
    )

    m.add_production(
        '''
        (p retrieve-from-location
           =goal>
              isa         sentence-goal
              arg2        =location
              state       get-retrieval
           ==>
           =goal>
              state       nil
           +retrieval>
              isa         comprehend-sentence
              arg2        =location)
        '''
    )

    m.add_production(
        '''
        (p respond-yes
           =goal>
              isa         sentence-goal
              arg1        =person
              arg2        =location
              state       nil
           =retrieval>
              isa         comprehend-sentence
              arg1        =person
              arg2        =location
           ==>
           =goal>
              state       "k")
        '''
    )

    m.add_production(
        '''
        (p mismatch-person-no
           =goal>
              isa         sentence-goal
              arg1        =person
              arg2        =location
              state       nil
           =retrieval>
              isa         comprehend-sentence
            - arg1        =person
           ==>
           =goal>
              state       "d")
        '''
    )

    m.add_production(
        '''
        (p mismatch-location-no
           =goal>
              isa         sentence-goal
              arg1        =person
              arg2        =location
              state       nil
           =retrieval>
              isa         comprehend-sentence
            - arg2        =location
           ==>
           =goal>
              state       "d")
        '''
    )
    m.spp('mismatch-location-no', **{'at': 0.21})
    m.spp('mismatch-person-no', **{'at': 0.21})
    m.spp('respond-yes', **{'at': 0.21})
    m.spp('start', **{'at': 0.25})
    m.spp('harvest-person', **{'at': 0.285})
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
