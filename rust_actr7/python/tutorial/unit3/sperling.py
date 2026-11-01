"""Python port of ACT-R:tutorial;unit3;sperling-model.lisp plus the sperling task."""

from __future__ import annotations

import json
import sys

import rust_actr.actr as actr

DEFAULT_FUNCTION = 'experiment'
DEFAULT_ARGS = [1]


def build(seed=(100, 0)) -> actr.Model:
    m = actr.Model('sperling', seed=seed)

    m.sgp(':v', True)
    m.sgp(':declarative-finst-span', 10)
    m.sgp(':show-focus', True)
    m.sgp(':trace-detail', 'low')
    m.sgp(':seed', [100, 0])

    m.chunk_type('read-letters', 'location', 'tone', 'step', 'upper-y', 'lower-y')
    m.chunk_type('report-row', 'row')

    m.add_chunk('attending', 'chunk')
    m.add_chunk('medium', 'chunk')
    m.add_chunk('find', 'chunk')
    m.add_chunk('encode', 'chunk')

    m.add_production(
        '''
        (p detected-sound
           =aural-location>
           ?aural>
             state   free
           ==>
           +aural>
             event   =aural-location)
        '''
    )

    m.add_production(
        '''
        (p sound-respond-low
           =goal>
             isa      read-letters
             tone     nil
           =aural>
             isa      sound
             content  500
        ==>
           =goal>
             tone     low
             upper-y  500
             lower-y  520)
        '''
    )

    m.add_production(
        '''
        (p sound-respond-medium
           =goal>
             isa      read-letters
             tone     nil
           =aural>
             isa      sound
             content  1000
        ==>
           =goal>
             tone     medium
             upper-y  450
             lower-y  470)
        '''
    )

    m.add_production(
        '''
        (p sound-respond-high
           =goal>
             isa      read-letters
             tone     nil
           =aural>
             isa      sound
             content  2000
        ==>
           =goal>
             tone     high
             upper-y  400
             lower-y  420)
        '''
    )

    m.add_production(
        '''
        (p attend-low
           =goal>
             isa        read-letters
             step       attending
           =visual-location>
             isa        visual-location
           > screen-y   500
           < screen-y   520

           ?visual>
             state      free
           ==>
           =goal>
             location   low
             step       encode
           +visual>
             cmd        move-attention
             screen-pos =visual-location)
        '''
    )

    m.add_production(
        '''
        (p attend-medium
           =goal>
             isa        read-letters
             step       attending
           =visual-location>
             isa        visual-location
           > screen-y   450
           < screen-y   470

           ?visual>
             state      free
        ==>
           =goal>
             location   medium
             step       encode
           +visual>
             cmd        move-attention
             screen-pos =visual-location)
        '''
    )

    m.add_production(
        '''
        (p attend-high
           =goal>
             isa        read-letters
             step       attending
           =visual-location>
             isa        visual-location
           > screen-y   400
           < screen-y   420

           ?visual>
             state      free
        ==>
           =goal>
             location   high
             step       encode
           +visual>
             cmd        move-attention
             screen-pos =visual-location)
        '''
    )

    m.add_production(
        '''
        (p encode-row-and-find
           =goal>
             isa       read-letters
             location  =pos
             upper-y   =uy
             lower-y   =ly
           =visual>
        ==>
           =visual>
             status    =pos
           -visual>
           =goal>
             location  nil
             step      attending
           +visual-location>
             :attended nil
           > screen-y  =uy
           < screen-y  =ly)
        '''
    )

    m.add_production(
        '''
        (p start-report
           =goal>
             isa     read-letters
             tone    =tone
           ?visual>
             state   free
           ==>
           +goal>
             isa     report-row
             row     =tone
           +retrieval>
             status  =tone)
        '''
    )

    m.add_production(
        '''
        (p do-report
           =goal>
             isa      report-row
             row      =tone
           =retrieval>
             status   =tone
             value    =val
           ?manual>
             state    free
           ==>
           +manual>
             cmd      press-key
             key      =val
           +retrieval>
             status   =tone
             :recently-retrieved  nil)
        '''
    )

    m.add_production(
        '''
        (p stop-report
           =goal>
             isa     report-row
             row     =row
           ?retrieval>
             buffer  failure
           ?manual>
             state   free
        ==>
           +manual>
             cmd     press-key
             key     space
           -goal>)
        '''
    )

    m.goal_focus(isa='read-letters', step='attending', upper_y=400, lower_y=600)

    m.spp('start-report', **{'u': -2})
    m.spp('detected-sound', **{'u': 10})
    m.spp('sound-respond-low', **{'u': 10})
    m.spp('sound-respond-medium', **{'u': 10})
    m.spp('sound-respond-high', **{'u': 10})
    return m


actr.clear_state()

# ACT-R tutorial unit 3 demonstration task.
# The sperling experiment displays a block of 12
# letters in the window, with 3 lines of 4 letters
# per line for a brief time.  After the display has
# been presented a tone sound is generated to indicate
# which row of letters must be reported (the timing of
# the tone relative to the initial display of the 
# letters is variable).  After the letters go away
# the participant must press the keys to indicate which
# letters were in the target row, and press the spacebar
# to indicate completion.

# Import the actr module for tutorial tasks


# Load the corresponding tutorial model

MODEL = actr.use(build())

# Define some global variables: responses holds the list of
# keys pressed by the participant, show_responses indicates
# whether or not to print out the responses provided after a
# trial is run, and exp_data holds the results of the
# original experiment (number of correct responses from
# the target row based on the delay of the tone) for 
# comparison to the model's performance.

responses = []
show_responses = True

exp_data = [3.03,2.4,2.03,1.5]

# The trial function runs a single trial of the task
# and returns the number of correct responses.  It requires one
# parameter which is the time in seconds to delay the tone 
# after the items have been presented.

def trial(onset_time):
   
    # Reset ACT-R and all models to initial state

    actr.reset()

    # create some local variables to perform the trial:
    #  letters: is a randomized list of letter strings
    #  answers: will be the list of letters in the target row
    #  row: a random number from 0-2 indicating which row will
    #       be the target
    #  window: an experiment window created to display the task

    letters = actr.permute_list(["B","C","D","F","G","H","J",
                                 "K","L","M","N","P","Q","R",
                                 "S","T","V","W","X","Y","Z"])
    answers = []
    row = actr.random(3)
    window = actr.open_exp_window("Sperling Experiment", visible=True)

    # Show the first 12 letters from the list in the window in
    # three rows of four and record which ones are in the target
    # row in the answers variable.

    for i in range(3):
        for j in range(4):
            txt = letters[j + (i * 4)]
            if i == row:
                answers.append(txt)
            actr.add_text_to_exp_window(window, txt, x=(75 + (j * 50)), y=(100 + (i * 50)))

    # Tell the model to interact with that window

    actr.install_device(window)

    # Set the freq variable based on which row is the target

    if row == 0:
        freq = 2000
    elif row == 1:
        freq = 1000
    else:
        freq = 500

    # Create a tone with frequency freq for .5 seconds
    # starting at the indicated onset_time

    actr.new_tone_sound(freq,.5,onset_time)

    # To simulate the persistent visual memory for the model
    # we will not clear the display until after a randomly
    # chosen time between .9 and 1.1 seconds has passed.
    # This is done by scheduling the clear-exp-window command
    # to be called after that amount of time has passed.

    actr.schedule_event_relative(900 + actr.random(200),
                                        "clear-exp-window",
                                        params=[window],time_in_ms=True)

    # clear the response variable 

    global responses
    responses = []

    # Add a command for our respond_to_key_press function so that
    # ACT-R can call it.

    actr.add_command("sperling-response",respond_to_key_press,
                     "Sperling task key press response monitor")

    # Monitor the output-key action so that our respond_to_key_press
    # function is called when a key is pressed.

    actr.monitor_command("output-key","sperling-response")

    # Run the model for up to 30 seconds in real time mode.

    actr.run(30,True)    

    # Stop monitoring the output-key action and remove our command.

    actr.remove_command_monitor("output-key","sperling-response")
    actr.remove_command("sperling-response")

    # If the show_responses variable is True then print out
    # the correct answers and the responses that were provided

    if show_responses:
        print("answers: %s"%answers)
        print("responses: %s"%responses)

    # Call the compute_score function to determine the number of
    # correct responses and return the result.

    return(compute_score(answers))

# The compute_score function counts how many of the correct answers
# were provided by the participant and returns that number.

def compute_score(answers):

    score = 0
 
    for s in responses:
        if s.upper() in answers:
            score += 1

    return(score)

# This function is the one that will be called when the participant
# presses a key, and it just records the result in the responses
# list unless it is the space bar.

def respond_to_key_press (model,key):
    global responses

    if not(key.lower() == "space"):
        responses.append(key)

# The report_data function takes a list of the average number of items
# reported in the target row ordered by onset delay. It compares those
# to the original experiment's data based on correlation and mean deviation
# then calls print_results to display the data.

def report_data(data):

    actr.correlation(data,exp_data)
    actr.mean_deviation(data,exp_data)
    print_results (data)

# The print_results function takes a list of the average target row data
# and prints that in a table along with the original experiment's data.

def print_results(data):

    print("Condition    Current Participant   Original Experiment")
    for (c,d,o) in zip([0.0,0.15,0.3,1.0],data,exp_data):
        print(" %4.2f sec.          %6.2f                %6.2f"%(c,d,o))


# The one_block function runs a trial of the experiment at each
# of the experiment's tone onset conditions in a random order.
# It returns a list of the correct answer counts in the order
# of onset duration (lowest first).

def one_block():

    result = []

    for t in actr.permute_list([0.0,.15,.3,1.0]):
        result.append((t,trial(t)))

    result.sort()
    return (list(map(lambda x: x[1],result)))


# The experiment function takes one required parameter which is
# the number of blocks to run in the experiment (where each block
# is one trial at each of the 4 possible onset times).  It collects
# the data over blocks, averages the results, and passes that to the
# report_data function for display.

def experiment(n):
    results=[0,0,0,0]

    for i in range(n):
        results=list(map(lambda x,y: x + y,results,one_block()))

    report_data(list(map(lambda x: x/n,results)))


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
