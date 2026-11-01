"""Python port of ACT-R:tutorial;unit5;fan-model.lisp plus the fan task."""

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

    m.add_production(
        '''
        (p find-person
           ?visual-location>
              buffer      unrequested
           ?imaginal>
              state       free
           ==>
           +imaginal>

           +visual-location>
              isa         visual-location
              screen-x    lowest)
        '''
    )

    m.add_production(
        '''
        (p attend-visual-location
           =visual-location>

           ?visual-location>
              buffer      requested
           ?visual>
              state       free
           ==>
           +visual>
              cmd         move-attention
              screen-pos  =visual-location)
        '''
    )

    m.add_production(
        '''
        (p retrieve-meaning
           =visual>
              isa         visual-object
              value       =word
           ==>
           +retrieval>
              isa         meaning
              word        =word)
        '''
    )

    m.add_production(
        '''
        (p encode-person
           =retrieval>

           =imaginal>
              isa         comprehend-sentence
              arg1        nil
           ==>
           =imaginal>
              arg1        =retrieval
           +visual-location>
              isa         visual-location
              screen-x    highest)
        '''
    )

    m.add_production(
        '''
        (p encode-location
           =retrieval>

           =imaginal>
              isa         comprehend-sentence
              arg1        =arg
              arg2        nil
           ==>
           =imaginal>
              arg2        =retrieval)
        '''
    )

    m.add_production(
        '''
        (p retrieve-from-person
           =imaginal>
              isa         comprehend-sentence
              arg1        =person
              arg2        =location
           ?retrieval>
              state       free
              buffer      empty
           ==>
           =imaginal>
           +retrieval>
              isa         comprehend-sentence
              arg1        =person)
        '''
    )

    m.add_production(
        '''
        (p retrieve-from-location
           =imaginal>
              isa         comprehend-sentence
              arg1        =person
              arg2        =location
           ?retrieval>
              state       free
              buffer      empty
           ==>
           =imaginal>
           +retrieval>
              isa         comprehend-sentence
              arg2        =location)
        '''
    )

    m.add_production(
        '''
        (p yes
           =imaginal>
              isa         comprehend-sentence
              arg1        =person
              arg2        =location
           =retrieval>
              isa         comprehend-sentence
              arg1        =person
              arg2        =location
           ?manual>
              state       free
           ==>
           +manual>
              cmd         press-key
              key         "k")
        '''
    )

    m.add_production(
        '''
        (p mismatch-person
           =imaginal>
              isa         comprehend-sentence
              arg1        =person
              arg2        =location
           =retrieval>
              isa         comprehend-sentence
            - arg1        =person
           ?manual>
              state       free
           ==>
           +manual>
              isa         press-key
              key         "d")
        '''
    )

    m.add_production(
        '''
        (p mismatch-location
           =imaginal>
              isa         comprehend-sentence
              arg1        =person
              arg2        =location
           =retrieval>
              isa         comprehend-sentence
            - arg2        =location
           ?manual>
              state       free
           ==>
           +manual>
              cmd         press-key
              key         "d")
        '''
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

actr.bind_module(globals(), MODEL)


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
