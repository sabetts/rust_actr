"""Python port of ACT-R:tutorial;unit5;siegler-model.lisp plus the siegler task."""

from __future__ import annotations

import json
import sys

import rust_actr.actr as actr

LOGICAL_PATH = 'ACT-R:tutorial;unit5;siegler-model.lisp'
MODEL_NAME = 'SIEGLER'
DEFAULT_FUNCTION = 'experiment'
DEFAULT_ARGS = [1]


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
