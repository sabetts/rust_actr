"""Python port of ACT-R:tutorial;unit3;subitize-model.lisp plus the subitize task."""

from __future__ import annotations

import json
import sys

import rust_actr.actr as actr

DEFAULT_FUNCTION = 'experiment'
DEFAULT_ARGS = []


def build(seed=(100, 0)) -> actr.Model:
    m = actr.Model('subitize', seed=seed)

    m.sgp('v', True)
    m.sgp('show_focus', True)
    m.sgp('visual_num_finsts', 10)
    m.sgp('visual_finst_span', 10)

    m.chunk_type('count', 'count', 'step')
    m.chunk_type('number', 'number', 'next', 'vocal-rep')

    m.add_chunk('zero', 'number', number='zero', next='one', vocal_rep=actr.String('zero'))
    m.add_chunk('one', 'number', number='one', next='two', vocal_rep=actr.String('one'))
    m.add_chunk('two', 'number', number='two', next='three', vocal_rep=actr.String('two'))
    m.add_chunk('three', 'number', number='three', next='four', vocal_rep=actr.String('three'))
    m.add_chunk('four', 'number', number='four', next='five', vocal_rep=actr.String('four'))
    m.add_chunk('five', 'number', number='five', next='six', vocal_rep=actr.String('five'))
    m.add_chunk('six', 'number', number='six', next='seven', vocal_rep=actr.String('six'))
    m.add_chunk('seven', 'number', number='seven', next='eight', vocal_rep=actr.String('seven'))
    m.add_chunk('eight', 'number', number='eight', next='nine', vocal_rep=actr.String('eight'))
    m.add_chunk('nine', 'number', number='nine', next='ten', vocal_rep=actr.String('nine'))
    m.add_chunk('ten', 'number', number='ten', next='eleven', vocal_rep=actr.String('ten'))
    m.add_chunk('eleven', 'number', number='eleven')
    m.add_dm('start')

    m.goal_focus(isa='count', step='start')
    return m


actr.clear_state()

# ACT-R tutorial unit3 subitize experiment.
# This experiment displays a number of Xs on
# the screen and the participant must respond 
# with how many are there. A human participant
# must press a key from 0-9 (where 0 represents 
# 10 items) but a model must speak the count 
# (which is how the original experiment was
# performed).  The time of the response and its
# correctness are recorded.

# Import the actr module for tutorial tasks


# Load the corresponding ACT-R starting model.

MODEL = actr.use(build())

# Create some global variables to hold the response
# given and the time it occurred

response = False
response_time = False

# A variable holding the data from the original experiment

exp_data = [.6,.65,.7,.86, 1.12,1.5,1.79,2.13,2.15,2.58]

# Two functions for converting an integer to a string 
# as either the word e.g. "one" or the digits e.g. "1"
# for comparison to the response given by a model (spoken)
# or a person (keypress).

def number_to_word (n):
    map = ['','one','two','three','four','five','six','seven','eight','nine','ten']
    return map[n]

def number_to_string (n):
    if n == 10:
        return "0"
    else:
        return str(n)

# The trial function presents one trial of the task.
# It requires one parameter which is the number of items to
# present (should be an integer from 1-10), and has an optional
# parameter which indicates whether it is the model or a person
# performing the task.  It returns a list with the time it took
# the participant to respond and True if the response was correct 
# or the list [30, False] if the response was incorrect or no
# response was provided.

def trial (n,human=False):

    # Reset ACT-R and all models to initial state

    actr.reset()

    # create some local variables to perform the trial:
    #  points: is a list of randomized x,y coordinates to display the Xs
    #          from the generate_points function
    #  window: an experiment window created to display the task
    #  start: the current time at the start of the trial 
    #         as given by the ACT-R get_time function which
    #         needs to be provided whether it is the model or
    #         a person doing the task to get the appropriate time

    points = generate_points(n)
    window = actr.open_exp_window("Subitizing Experiment")
    start = actr.get_time(not(human))
   
    # Display an x at each of the points
 
    for point in points:
        actr.add_text_to_exp_window(window, "x", x=point[0], y=point[1])

    # clear the response variables

    global response,response_time

    response = ''
    response_time = False

    # Run the trial

    if human:
        if actr.visible_virtuals_available():

            # If a human is doing the task and there is a visible
            # window available for them to interact with then
            # add a command and monitor the output-key action

            actr.add_command("subitize-response",respond_to_key_press,
                             "Subitize task human response")
            actr.monitor_command("output-key","subitize-response")

            # Set the correct answer string for a key press

            answer = number_to_string(n)

            # Wait until there is a response

            while response == '':
                actr.process_events()

            # Stop monitoring output-key and remove the command

            actr.remove_command_monitor("output-key","subitize-response")
            actr.remove_command("subitize-response")
    else:

        # If a model is doing the task add a command and
        # monitor the output-speech action

        actr.add_command("subitize-response",record_model_speech,
                         "Subitize task model response")
        actr.monitor_command("output-speech","subitize-response")

        # Set the correct answer string for a spoken response

        answer = number_to_word(n)

        # Tell the model to interact with the created window

        actr.install_device(window)

        # Run the model for up to 30 seconds in real time mode

        actr.run(30,True)

        # Stop monitoring output-speech and remove the command

        actr.remove_command_monitor("output-speech","subitize-response")
        actr.remove_command("subitize-response")

    # If a response is given and it matches the correct answer
    # then return a list with the time since the trial started
    # in seconds and True, otherwise return a list of 30 and False

    if response != '' and response.lower() == answer.lower():
        return [(response_time - start) / 1000.0, True]
    else:
        return [30, False]

# experiment takes one optional parameter which indicates
# whether it is a human or model performing the task.  Then it 
# presents a trial for each of the counts from 1-10 in a randomized
# order, and passes the results to report_data sorted by item count.

def experiment(human=False):

    results = []
    for items in actr.permute_list([10,9,8,7,6,5,4,3,2,1]):
      results.append((items,trial(items,human)))

    results.sort()
    report_data(list(map(lambda x: x[1],results)))

# report-data compares the times in the provided data to
# the original experiment data and then passes it to print_results
# for output.

def report_data(data):

    rts = list(map(lambda x: x[0],data))
    actr.correlation(rts,exp_data)
    actr.mean_deviation(rts,exp_data)
    print_results(data)

# print_results outputs a table with the times and correctness
# values from the current experiment along with the data from
# the origial experiment.

def print_results(data):

    print("Items    Current Participant   Original Experiment")
    for count, d, original in zip(range(len(data)),data,exp_data):
        print("%3d        %5.2f  (%-5s)            %5.2f" % 
              (count+1,d[0],d[1],original))

# The next three functions: generate_points, new_distinct_point, and
# too_close are used to generate a list of random x,y lists for the
# coordinates to display the Xs so that they are within the bounds of
# the window and non-overlapping.

def generate_points(n):
    p=[]
    for i in range(n):
        p.append(new_distinct_point(p))
    return p

def new_distinct_point(points):

    while True:
        x = actr.random(240)+20
        y = actr.random(240)+20
        if not(any(too_close(x,y,p) for p in points)):
            break;
    return [x,y]

def too_close (x,y,p):
    if (abs(x-p[0]) < 40) and (abs(y-p[1]) < 40):
        return True
    else:
        return False


# respond_to_key_press is monitoring output-key to record 
# the current time and key pressed when a human is performing
# the task.

def respond_to_key_press (model,key):
    global response,response_time

    response_time = actr.get_time(False)
    response = key


# record_model_speech is monitoring output-speech to record 
# the current time and word spoken when a model is performing
# the task.

def record_model_speech (model,string):
    global response,response_time

    response_time = actr.get_time(True)
    response = string


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
