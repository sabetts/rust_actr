"""Python port of ACT-R:tutorial;unit5;siegler-model.lisp plus the siegler task."""

from __future__ import annotations

import json
import sys

import rust_actr.actr as actr

DEFAULT_FUNCTION = 'experiment'
DEFAULT_ARGS = [1]


def build(seed=(100, 0)) -> actr.Model:
    m = actr.Model('siegler', seed=seed)

    m.sgp('rt', -0.45)
    m.sgp('esc', True)
    m.sgp('v', None)
    m.sgp('act', None)
    m.sgp('ans', 0.5)
    m.sgp('mp', 16)

    m.chunk_type('plus-fact', 'addend1', 'addend2', 'sum')
    m.chunk_type('number', 'aural-rep', 'vocal-rep')

    m.add_chunk('zero', 'number', aural_rep=0, vocal_rep=actr.String('zero'))
    m.add_chunk('one', 'number', aural_rep=1, vocal_rep=actr.String('one'))
    m.add_chunk('two', 'number', aural_rep=2, vocal_rep=actr.String('two'))
    m.add_chunk('three', 'number', aural_rep=3, vocal_rep=actr.String('three'))
    m.add_chunk('four', 'number', aural_rep=4, vocal_rep=actr.String('four'))
    m.add_chunk('five', 'number', aural_rep=5, vocal_rep=actr.String('five'))
    m.add_chunk('six', 'number', aural_rep=6, vocal_rep=actr.String('six'))
    m.add_chunk('seven', 'number', aural_rep=7, vocal_rep=actr.String('seven'))
    m.add_chunk('eight', 'number', aural_rep=8, vocal_rep=actr.String('eight'))
    m.add_chunk('nine', 'number', aural_rep=9, vocal_rep=actr.String('nine'))
    m.add_chunk('f00', 'plus-fact', addend1='zero', addend2='zero', sum='zero')
    m.add_chunk('f01', 'plus-fact', addend1='zero', addend2='one', sum='one')
    m.add_chunk('f02', 'plus-fact', addend1='zero', addend2='two', sum='two')
    m.add_chunk('f03', 'plus-fact', addend1='zero', addend2='three', sum='three')
    m.add_chunk('f04', 'plus-fact', addend1='zero', addend2='four', sum='four')
    m.add_chunk('f05', 'plus-fact', addend1='zero', addend2='five', sum='five')
    m.add_chunk('f10', 'plus-fact', addend1='one', addend2='zero', sum='one')
    m.add_chunk('f11', 'plus-fact', addend1='one', addend2='one', sum='two')
    m.add_chunk('f12', 'plus-fact', addend1='one', addend2='two', sum='three')
    m.add_chunk('f13', 'plus-fact', addend1='one', addend2='three', sum='four')
    m.add_chunk('f14', 'plus-fact', addend1='one', addend2='four', sum='five')
    m.add_chunk('f15', 'plus-fact', addend1='one', addend2='five', sum='six')
    m.add_chunk('f20', 'plus-fact', addend1='two', addend2='zero', sum='two')
    m.add_chunk('f21', 'plus-fact', addend1='two', addend2='one', sum='three')
    m.add_chunk('f22', 'plus-fact', addend1='two', addend2='two', sum='four')
    m.add_chunk('f23', 'plus-fact', addend1='two', addend2='three', sum='five')
    m.add_chunk('f24', 'plus-fact', addend1='two', addend2='four', sum='six')
    m.add_chunk('f25', 'plus-fact', addend1='two', addend2='five', sum='seven')
    m.add_chunk('f30', 'plus-fact', addend1='three', addend2='zero', sum='three')
    m.add_chunk('f31', 'plus-fact', addend1='three', addend2='one', sum='four')
    m.add_chunk('f32', 'plus-fact', addend1='three', addend2='two', sum='five')
    m.add_chunk('f33', 'plus-fact', addend1='three', addend2='three', sum='six')
    m.add_chunk('f34', 'plus-fact', addend1='three', addend2='four', sum='seven')
    m.add_chunk('f35', 'plus-fact', addend1='three', addend2='five', sum='eight')
    m.add_chunk('f40', 'plus-fact', addend1='four', addend2='zero', sum='four')
    m.add_chunk('f41', 'plus-fact', addend1='four', addend2='one', sum='five')
    m.add_chunk('f42', 'plus-fact', addend1='four', addend2='two', sum='six')
    m.add_chunk('f43', 'plus-fact', addend1='four', addend2='three', sum='seven')
    m.add_chunk('f44', 'plus-fact', addend1='four', addend2='four', sum='eight')
    m.add_chunk('f45', 'plus-fact', addend1='four', addend2='five', sum='nine')
    m.add_chunk('f50', 'plus-fact', addend1='five', addend2='zero', sum='five')
    m.add_chunk('f51', 'plus-fact', addend1='five', addend2='one', sum='six')
    m.add_chunk('f52', 'plus-fact', addend1='five', addend2='two', sum='seven')
    m.add_chunk('f53', 'plus-fact', addend1='five', addend2='three', sum='eight')
    m.add_chunk('f54', 'plus-fact', addend1='five', addend2='four', sum='nine')

    m.add_production(
        '''
        (p hear-sound
           =aural-location>
             isa        audio-event
             location   external
           ?aural>
             state      free
          ==>
           +aural>
             event      =aural-location)
        '''
    )

    m.add_production(
        '''
        (p encode-digit
           =aural>
             isa        sound
             content    =val
           ?retrieval>
             state      free
          ==>
           +retrieval>
             isa        number
             aural-rep  =val)
        '''
    )

    m.add_production(
        '''
        (p harvest-arg1
           =retrieval>
           ?imaginal>
             buffer     empty
             state      free
          ==>
           +imaginal>
             isa        plus-fact
             addend1    =retrieval)
        '''
    )

    m.add_production(
        '''
        (p harvest-arg2
           =retrieval>
           =imaginal>
             isa          plus-fact
             addend2      nil
           ?imaginal>
             state        free
          ==>
           *imaginal>
             addend2      =retrieval)
        '''
    )

    m.add_production(
        '''
        (p retrieve-answer
           =imaginal>
             isa          plus-fact
             addend1      =val1
             addend2      =val2
             sum          nil
           ?retrieval>
             state        free
             buffer       empty
          ==>
           =imaginal>

           +retrieval>
             isa          plus-fact
             addend1      =val1
             addend2      =val2)
        '''
    )

    m.add_production(
        '''
        (p harvest-answer
           =retrieval>
              isa         plus-fact
              sum         =number
           =imaginal>
              isa         plus-fact
           ?imaginal>
              state       free
          ==>
           *imaginal>
              sum         =number
           +retrieval>    =number)
        '''
    )

    m.add_production(
        '''
        (p respond
           =imaginal>
             isa         plus-fact
             sum         =result
           =retrieval>
             isa         number
             vocal-rep   =name
           ?vocal>
             state       free
          ==>
           +vocal>
             cmd         speak
             string      =name)
        '''
    )

    m.set_base_levels([('zero', 10), ('one', 10), ('two', 10), ('three', 10), ('four', 10), ('five', 10), ('six', 10), ('seven', 10), ('eight', 10), ('nine', 10)])
    m.set_similarities([('zero', 'one', -0.1), ('one', 'two', -0.1), ('two', 'three', -0.1), ('three', 'four', -0.1), ('four', 'five', -0.1), ('zero', 'two', -0.2), ('one', 'three', -0.2), ('two', 'four', -0.2), ('three', 'five', -0.2), ('zero', 'three', -0.3), ('one', 'four', -0.3), ('two', 'five', -0.3), ('zero', 'four', -0.4), ('one', 'five', -0.4), ('zero', 'five', -0.5)])
    m.set_base_levels([('f00', 0.65), ('f01', 0.65), ('f02', 0.65), ('f03', 0.65), ('f04', 0.65), ('f10', 0.65), ('f11', 0.65), ('f12', 0.65), ('f13', 0.65), ('f20', 0.65), ('f21', 0.65), ('f22', 0.65), ('f30', 0.65), ('f31', 0.65), ('f40', 0.65)])
    return m


actr.clear_state()

# ACT-R tutorial unit 5 siegler task.
# This experiment presents a model with a pair of numbers aurally
# and the model must respond vocally with the sum of those numbers.
# The task and data to which the model is fit are in the paper:
#
# Siegler, R. S., & Shrager, J. (1984). Strategy choices in addition 
# and subtraction: How do children know what to do? In C. Sophian (Ed.), 
# Origins of cognitive skills (pp. 229-293). Hillsdale, NJ: Erlbaum.
#
# The original experiment was performed with 4 year-olds who made
# many errors in their responses.

# Import the actr module for tutorial tasks


# Load the corresponding model for the task.

MODEL = actr.use(build())

# Create variables for the response, to record whether the 
# monitoring function is currently available, and a subset of
# the original data for comparison.

response = False
monitor_installed = False

siegler_data = [[0, .05, .86,  0,  .02,  0, .02, 0, 0, .06],
                [0, .04, .07, .75, .04,  0, .02, 0, 0, .09],
                [0, .02, 0, .10, .75, .05, .01, .03, 0, .06],
                [.02, 0, .04, .05, .80, .04, 0, .05, 0, 0],
                [0, 0, .07, .09, .25, .45, .08, .01, .01, .06],
                [.04, 0, 0, .05, .21, .09, .48, 0, .02, .11]]


# record_model_speech will be monitoring the output-speech
# command called by the microphone device so that it can
# record the model's speech output.

def record_model_speech (model,string):
    global response
    response = string.lower()


# Because the task can be run as a single trial, or over
# larger blocks it's more efficient to only install and
# remove the monitor once for the run instead of on each
# trial as has been done in other tasks.  These functions
# are used to do that when necessary.

def add_speech_monitor():
    global monitor_installed

    if monitor_installed == False:
        actr.add_command("siegler-response",record_model_speech,"Siegler task model response")
        actr.monitor_command("output-speech","siegler-response")
        monitor_installed = True
        return True
    else:
        return False

def remove_speech_monitor():

    actr.remove_command_monitor("output-speech","siegler-response")
    actr.remove_command("siegler-response")

    global monitor_installed
    monitor_installed = False

# trial takes two parameters which must be numbers.
# It resets the model and adds a microphone device to record
# the models speech output.  Then, the numbers are presented
# aurally to the model using new-digit-sound, and after 
# running the model it returns any vocal response that it made.

def trial(arg1,arg2):

    actr.reset()
    actr.install_device(["speech","microphone"])
    need_to_remove = add_speech_monitor()
    actr.new_digit_sound(arg1)
    actr.new_digit_sound(arg2,.75)
    global response
    response = False
    actr.run(30)
    if need_to_remove:
        remove_speech_monitor()

    return response

# set runs one trial for each of the addition problems
# in the data set and returns the results of those trials.

def set ():

    need_to_remove = add_speech_monitor()
    
    data = [trial(1,1),trial(1,2),trial(1,3),
            trial(2,2),trial(2,3),trial(3,3)]

    if need_to_remove:
        remove_speech_monitor()

    return data

# experiment requires one parameter which is how many sets
# of trials to run.  It runs that many trials collecting the
# responses and then passes those to analyze to compute the 
# response percentages, compare the data to the experimental 
# data, and display the results.

def experiment(n):
    
    add_speech_monitor()

    data = []

    for i in range(n):
        data.append(set())

    remove_speech_monitor()
    analyze(data)


def analyze(responses):

    results = [[0,0,0,0,0,0,0,0,0,0],
               [0,0,0,0,0,0,0,0,0,0],
               [0,0,0,0,0,0,0,0,0,0],
               [0,0,0,0,0,0,0,0,0,0],
               [0,0,0,0,0,0,0,0,0,0],
               [0,0,0,0,0,0,0,0,0,0]]

    positions = {'zero':0,'one':1,'two':2,'three':3,'four':4,'five':5,
                 'six':6,'seven':7,'eight':8}


    for r in responses:
        for i in range(6):
            if r[i] in positions:
                results[i][positions[r[i]]] += 1
            else:
                results[i][9] += 1

    n = len(responses)

    for i in range(6):
        for j in range(10):
            results[i][j] /= n

    display_results(results)

def display_results(results):

    questions = ["1+1","1+2","1+3","2+2","2+3","3+3"]

    actr.correlation(results,siegler_data)
    actr.mean_deviation(results,siegler_data)


    print("       0     1     2     3     4     5     6     7     8   Other")
    for i in range(6):
        print(questions[i],end="")
        
        for j in range(10):
            print("%6.2f" % results[i][j],end="")
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
