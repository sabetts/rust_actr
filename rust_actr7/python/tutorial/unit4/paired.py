"""Python port of ACT-R:tutorial;unit4;paired-model.lisp plus the paired task."""

from __future__ import annotations

import json
import sys

import rust_actr.actr as actr

LOGICAL_PATH = 'ACT-R:tutorial;unit4;paired-model.lisp'
MODEL_NAME = 'PAIRED'
DEFAULT_FUNCTION = 'experiment'
DEFAULT_ARGS = [1]


def build(seed=(100, 0)) -> actr.Model:
    m = actr.Model('PAIRED', seed=seed)
    m.sgp(':V', True)
    m.sgp(':ESC', True)
    m.sgp(':RT', -2)
    m.sgp(':LF', 0.4)
    m.sgp(':ANS', 0.5)
    m.sgp(':BLL', 0.5)
    m.sgp(':ACT', None)
    m.sgp(':NCNAR', None)
    m.sgp(':SEED', [200, 4])
    m.chunk_type('GOAL', 'STEP')
    m.chunk_type('PAIR', 'PROBE', 'ANSWER')
    m.add_chunk('START', 'CHUNK', **{})
    m.add_chunk('ATTENDING-TARGET', 'CHUNK', **{})
    m.add_chunk('ATTENDING-PROBE', 'CHUNK', **{})
    m.add_chunk('TESTING', 'CHUNK', **{})
    m.add_chunk('READ-STUDY-ITEM', 'CHUNK', **{})
    m.add_production(
        '''(P ATTEND-PROBE =GOAL> ISA GOAL STEP START =VISUAL-LOCATION> ?VISUAL> STATE FREE ==> +VISUAL> CMD MOVE-ATTENTION SCREEN-POS =VISUAL-LOCATION =GOAL> STEP ATTENDING-PROBE)'''
    )
    m.add_production(
        '''(P READ-PROBE =GOAL> ISA GOAL STEP ATTENDING-PROBE =VISUAL> ISA VISUAL-OBJECT VALUE =VAL ?IMAGINAL> STATE FREE ==> +IMAGINAL> ISA PAIR PROBE =VAL +RETRIEVAL> ISA PAIR PROBE =VAL =GOAL> STEP TESTING)'''
    )
    m.add_production(
        '''(P RECALL =GOAL> ISA GOAL STEP TESTING =RETRIEVAL> ISA PAIR ANSWER =ANS ?MANUAL> STATE FREE ?VISUAL> STATE FREE ==> +MANUAL> CMD PRESS-KEY KEY =ANS =GOAL> STEP READ-STUDY-ITEM +VISUAL> CMD CLEAR)'''
    )
    m.add_production(
        '''(P CANNOT-RECALL =GOAL> ISA GOAL STEP TESTING ?RETRIEVAL> BUFFER FAILURE ?VISUAL> STATE FREE ==> =GOAL> STEP READ-STUDY-ITEM +VISUAL> CMD CLEAR)'''
    )
    m.add_production(
        '''(P DETECT-STUDY-ITEM =GOAL> ISA GOAL STEP READ-STUDY-ITEM =VISUAL-LOCATION> ?VISUAL> STATE FREE ==> +VISUAL> CMD MOVE-ATTENTION SCREEN-POS =VISUAL-LOCATION =GOAL> STEP ATTENDING-TARGET)'''
    )
    m.add_production(
        '''(P ASSOCIATE =GOAL> ISA GOAL STEP ATTENDING-TARGET =VISUAL> ISA VISUAL-OBJECT VALUE =VAL =IMAGINAL> ISA PAIR PROBE =PROBE ?VISUAL> STATE FREE ==> =IMAGINAL> ANSWER =VAL -IMAGINAL> =GOAL> STEP START +VISUAL> CMD CLEAR)'''
    )
    m.goal_focus(isa='GOAL', **{'STEP': 'START'})
    return m


actr.clear_state()

# ACT-R tutorial unit 4 paired associate task.
# This experiment runs several trials of presenting a 
# word prompt, and waiting 5 seconds for a response,
# then it displays the correct response (which will be
# a digit) and waits 5 seconds before starting the next
# trial.  The time to respond to the initial prompt and
# the correctness of the response are recorded for
# comparison to the humam data of the task that had
# 20 different pairs (the digits were each paired with
# two words) over 8 blocks (a randomized ordering of the
# 20 pairs).

# Import the actr module for tutorial tasks


# Load the corresponding model for the task

MODEL = actr.use(build())

# Global variables to hold the participant's response, the time of
# that response, the possible stimuli, and the data from the 
# original experiment.

response = False
response_time = False

pairs = list(zip(['bank','card','dart','face','game','hand','jack','king','lamb','mask',
             'neck','pipe','quip','rope','sock','tent','vent','wall','xray','zinc'],
            ['0','1','2','3','4','5','6','7','8','9','0','1','2','3','4','5','6','7','8','9']))

latencies = [0.0, 2.158, 1.967, 1.762, 1.680, 1.552, 1.467, 1.402]
probabilities = [0.0, .526, .667, .798, .887, .924, .958, .954]

# task takes two required parameters: the number of stimuli to
# present in a block and the number of blocks to run.  The optional 
# parameter if specified as True will run a person instead of the model.

def task (size,trials,human=False):

    # Create a command for the respond_to_key_press function so that
    # it can be used to monitor "output-key".

    actr.add_command("paired-response",respond_to_key_press,
                     "Paired associate task key press response monitor")
    actr.monitor_command("output-key","paired-response")

    # Run the function that does the actual experiment, remove the monitor
    # and command that were added, and return the results.

    result = do_experiment(size,trials,human)

    actr.remove_command_monitor("output-key","paired-response")
    actr.remove_command("paired-response")

    return result

# respond_to_key_press will record the time of a key press
# using actr.get_time (which reports model time if passed a true
# value or real time if passed None/False, and model will be None
# if it is a person performing the task) and the key that 
# was pressed.

def respond_to_key_press (model,key):
    global response,response_time

    response_time = actr.get_time(model)
    
    response = key

# do_experiment takes three parameters, the number of pairs to
# present per block, the number of blocks, and whether it is a
# person performing the task.  It runs the number of blocks
# requested collecting the correctness and timing data per
# block, and then returns a list with lists where each sublist
# represents a block with the first item being % correct and
# the second mean response time.

    
def do_experiment(size, trials, human):

    if human and not(actr.visible_virtuals_available()):
        actr.print_warning("Cannot run the task as a person without a visible window available.")
    else:

        actr.reset()

       # create a varaible to hold the data, an indication of whether the
       # model is performing the task, and an opened experiment window that
       # is visible if a human is performing the task or virtual if it is
       # a model.

        result = []
        model = not(human)
        window = actr.open_exp_window("Paired-Associate Experiment", visible=human)

        if model:
            actr.install_device(window)

        # loop over the number of blocks
        
        for i in range(trials):
            score = 0
            time = 0

            # randomize the list of items to present which are
            # taken from the possible pairs

            for prompt,associate in actr.permute_list(pairs[20 - size:]):
         
                # clear the window and display the prompt

                actr.clear_exp_window(window)
                actr.add_text_to_exp_window (window, prompt, x=150 , y=150)

                # clear the response and record the time when the trial
                # is started

                global response
                response = ''
                start = actr.get_time(model)

                # If it's the model run it for exactly 5 seconds
                # and if it's a person wait for 5 seconds of real
                # time to pass.

                if model:
                    actr.run_full_time(5)
                else:
                    while (actr.get_time(False) - start) < 5000:
                        actr.process_events()

                # If there is a correct response increment the
                # count of correct answers and the cumulative
                # response times.

                if response == associate:
                    score += 1
                    time += response_time - start
            
                # Clear the window and display the correct response

                actr.clear_exp_window(window)
                actr.add_text_to_exp_window (window, associate, x=150 , y=150)
                start = actr.get_time(model)
           
                # If it's the model run it for exactly 5 seconds
                # and if it's a person wait for 5 seconds of real
                # time to pass.

                if model:
                    actr.run_full_time(5)
                else:
                    while (actr.get_time(False) - start) < 5000:
                        actr.process_events()

            # Record the score and time data in the result list

            if score > 0:
                average_time = time / score / 1000.0
            else:
                average_time = 0

            result.append((score/size,average_time))

        return result


# experiment takes one required parameter which is the number of times
# to run a model through the original experiment (20 pairs for 8 trials each).
# It collects the data from those trials which is passed to the output_data
# function for averaging and comparison to the original data.


def experiment(n):
    
    for i in range(n):

        if i == 0:
            data = task(20,8)
        else:
            data = list(map(lambda x,y: (x[0] + y[0],x[1] + y[1]),data,task(20,8)))

    output_data(data,n)

# output_data takes two required parameters which are a list of cumulative
# data items from the experiment and the number of experiment repetitions
# that were collected. It averages the results of the latency and accuracy
# data and calls print_results to display the comparison to the original
# data along with the average results.

def output_data(data,n):

    print_results(list(map(lambda x: x[1]/n,data)),latencies,"Latency")
    print_results(list(map(lambda x: x[0]/n,data)),probabilities,"Accuracy")


def print_results(predicted,data,label):

    print()
    print(label)
    actr.correlation(predicted,data)
    actr.mean_deviation(predicted,data)
    print("Trial    1       2       3       4       5       6       7       8")
    print("     ",end='')
    for i in predicted:
        print('%8.3f' % i,end='')
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
