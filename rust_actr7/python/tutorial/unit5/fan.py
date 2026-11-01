"""Python port of ACT-R:tutorial;unit5;fan-model.lisp plus the fan task."""

from __future__ import annotations

import json
import sys

import rust_actr.actr as actr

LOGICAL_PATH = 'ACT-R:tutorial;unit5;fan-model.lisp'
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
    m.add_production(
        '''(P FIND-PERSON ?VISUAL-LOCATION> BUFFER UNREQUESTED ?IMAGINAL> STATE FREE ==> +IMAGINAL> +VISUAL-LOCATION> ISA VISUAL-LOCATION SCREEN-X LOWEST)'''
    )
    m.add_production(
        '''(P ATTEND-VISUAL-LOCATION =VISUAL-LOCATION> ?VISUAL-LOCATION> BUFFER REQUESTED ?VISUAL> STATE FREE ==> +VISUAL> CMD MOVE-ATTENTION SCREEN-POS =VISUAL-LOCATION)'''
    )
    m.add_production(
        '''(P RETRIEVE-MEANING =VISUAL> ISA VISUAL-OBJECT VALUE =WORD ==> +RETRIEVAL> ISA MEANING WORD =WORD)'''
    )
    m.add_production(
        '''(P ENCODE-PERSON =RETRIEVAL> =IMAGINAL> ISA COMPREHEND-SENTENCE ARG1 NIL ==> =IMAGINAL> ARG1 =RETRIEVAL +VISUAL-LOCATION> ISA VISUAL-LOCATION SCREEN-X HIGHEST)'''
    )
    m.add_production(
        '''(P ENCODE-LOCATION =RETRIEVAL> =IMAGINAL> ISA COMPREHEND-SENTENCE ARG1 =ARG ARG2 NIL ==> =IMAGINAL> ARG2 =RETRIEVAL)'''
    )
    m.add_production(
        '''(P RETRIEVE-FROM-PERSON =IMAGINAL> ISA COMPREHEND-SENTENCE ARG1 =PERSON ARG2 =LOCATION ?RETRIEVAL> STATE FREE BUFFER EMPTY ==> =IMAGINAL> +RETRIEVAL> ISA COMPREHEND-SENTENCE ARG1 =PERSON)'''
    )
    m.add_production(
        '''(P RETRIEVE-FROM-LOCATION =IMAGINAL> ISA COMPREHEND-SENTENCE ARG1 =PERSON ARG2 =LOCATION ?RETRIEVAL> STATE FREE BUFFER EMPTY ==> =IMAGINAL> +RETRIEVAL> ISA COMPREHEND-SENTENCE ARG2 =LOCATION)'''
    )
    m.add_production(
        '''(P YES =IMAGINAL> ISA COMPREHEND-SENTENCE ARG1 =PERSON ARG2 =LOCATION =RETRIEVAL> ISA COMPREHEND-SENTENCE ARG1 =PERSON ARG2 =LOCATION ?MANUAL> STATE FREE ==> +MANUAL> CMD PRESS-KEY KEY "k")'''
    )
    m.add_production(
        '''(P MISMATCH-PERSON =IMAGINAL> ISA COMPREHEND-SENTENCE ARG1 =PERSON ARG2 =LOCATION =RETRIEVAL> ISA COMPREHEND-SENTENCE - ARG1 =PERSON ?MANUAL> STATE FREE ==> +MANUAL> ISA PRESS-KEY KEY "d")'''
    )
    m.add_production(
        '''(P MISMATCH-LOCATION =IMAGINAL> ISA COMPREHEND-SENTENCE ARG1 =PERSON ARG2 =LOCATION =RETRIEVAL> ISA COMPREHEND-SENTENCE - ARG2 =LOCATION ?MANUAL> STATE FREE ==> +MANUAL> CMD PRESS-KEY KEY "d")'''
    )
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
# This version of the task presents the probe items in a window
# which the model must read to complete the task.

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

# create variables to hold the model's response and the time of
# that response.

response = False
response_time = False


# The sentence function takes 4 parameters.
# The first two are the strings of the person and location
# to present.  The third is True or False to indicate whether
# this was or wasn't in the study set, and the last is
# either the string 'person' or 'location' to indicate which
# of the productions the model should use for retrieval.
#
# It presents the probe items given in a window, runs the 
# model, and returns a tuple indicating how many seconds it 
# took to respond (or 30 if no response was made) and True or False 
# to indicate if the response was correct.

def sentence (person, location, target, term):

    actr.reset()

    window = actr.open_exp_window("Sentence Experiment",visible=False,width=600,height=300)
    x = 25

    actr.install_device(window)

    actr.add_command("fan-response",respond_to_key_press,"Fan experiment model response")
    actr.monitor_command("output-key","fan-response")

    # disable the production that isn't being used for retrieval

    if term == 'person':
        actr.pdisable("retrieve-from-location")
    else:
        actr.pdisable("retrieve-from-person")


    actr.add_text_to_exp_window (window, person, x=50, y=150, width=75)
    actr.add_text_to_exp_window (window, location, x=250, y=150, width=75)

    global response,response_time

    response = ''
    response_time = 0

    actr.run(30)

    actr.remove_command_monitor("output-key","fan-response")
    actr.remove_command("fan-response")

    if response == '':
        return (30,False)
    elif target:
        if response.lower() == 'k'.lower():
            return (response_time / 1000,True)
        else:
            return (response_time / 1000,False)
    else:
        if response.lower() == 'd'.lower():
            return (response_time / 1000,True)
        else:
            return (response_time / 1000,False)
   

# respond_to_key_press is set to monitor the output-key command
# and records the time and key that was pressed by the model.

def respond_to_key_press (model,key):
    global response,response_time

    response_time = actr.get_time()
    response = key

# do_person_location requires one parameter which is either
# the string 'person' or 'location' to indicate which of the 
# productions the model should use for retrieval.
# It runs one trial of each fan condition and returns a list
# of the results.
    
def do_person_location(term):

    results = []

    for person,location,target in [("lawyer", "store", True),
                                   ("captain", "cave", True),
                                   ("hippie", "church", True),
                                   ("debutante", "bank", True),
                                   ("earl", "castle", True),
                                   ("hippie", "bank", True),
                                   ("fireman", "park", True),
                                   ("captain", "park", True),
                                   ("hippie", "park", True),
                                   ("fireman", "store", False),
                                   ("captain", "store", False),
                                   ("giant", "store", False),
                                   ("fireman", "bank", False),
                                   ("captain", "bank", False),
                                   ("giant", "bank", False),
                                   ("lawyer", "park", False),
                                   ("earl", "park", False),
                                   ("giant", "park", False)]:

        results.append(sentence(person,location,target,term))

    return results

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

    print("TARGETS:\n                         Person fan")
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
