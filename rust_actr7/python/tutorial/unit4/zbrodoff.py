"""Python port of ACT-R:tutorial;unit4;zbrodoff-model.lisp plus the zbrodoff task."""

from __future__ import annotations

import json
import sys

import rust_actr.actr as actr

DEFAULT_FUNCTION = 'experiment'
DEFAULT_ARGS = []


def build(seed=(100, 0)) -> actr.Model:
    m = actr.Model('zbrodoff', seed=seed)

    m.sgp('v', None)
    m.sgp('esc', True)
    m.sgp('lf', 0.4)
    m.sgp('bll', 0.5)
    m.sgp('ans', 0.5)
    m.sgp('rt', 0)
    m.sgp('ncnar', None)
    m.sgp('hear_self', None)
    m.sgp('show_focus', True)

    m.chunk_type('problem', 'arg1', 'arg2', 'result')
    m.chunk_type('goal', 'state', 'count', 'target', 'next-let', 'next-num')
    m.chunk_type('number', 'number', 'next', 'visual-rep', 'vocal-rep')
    m.chunk_type('letter', 'letter', 'next', 'visual-rep', 'vocal-rep')

    m.add_chunk('zero', 'number', number='zero', next='one', visual_rep=actr.String('0'), vocal_rep=actr.String('zero'))
    m.add_chunk('one', 'number', number='one', next='two', visual_rep=actr.String('1'), vocal_rep=actr.String('one'))
    m.add_chunk('two', 'number', number='two', next='three', visual_rep=actr.String('2'), vocal_rep=actr.String('two'))
    m.add_chunk('three', 'number', number='three', next='four', visual_rep=actr.String('3'), vocal_rep=actr.String('three'))
    m.add_chunk('four', 'number', number='four', next='five', visual_rep=actr.String('4'), vocal_rep=actr.String('four'))
    m.add_chunk('five', 'number', number='five')
    m.add_chunk('a', 'letter', letter='a', next='b', visual_rep=actr.String('a'), vocal_rep=actr.String('a'))
    m.add_chunk('b', 'letter', letter='b', next='c', visual_rep=actr.String('b'), vocal_rep=actr.String('b'))
    m.add_chunk('c', 'letter', letter='c', next='d', visual_rep=actr.String('c'), vocal_rep=actr.String('c'))
    m.add_chunk('d', 'letter', letter='d', next='e', visual_rep=actr.String('d'), vocal_rep=actr.String('d'))
    m.add_chunk('e', 'letter', letter='e', next='f', visual_rep=actr.String('e'), vocal_rep=actr.String('e'))
    m.add_chunk('f', 'letter', letter='f', next='g', visual_rep=actr.String('f'), vocal_rep=actr.String('f'))
    m.add_chunk('g', 'letter', letter='g', next='h', visual_rep=actr.String('g'), vocal_rep=actr.String('g'))
    m.add_chunk('h', 'letter', letter='h', next='i', visual_rep=actr.String('h'), vocal_rep=actr.String('h'))
    m.add_chunk('i', 'letter', letter='i', next='j', visual_rep=actr.String('i'), vocal_rep=actr.String('i'))
    m.add_chunk('j', 'letter', letter='j', next='k', visual_rep=actr.String('j'), vocal_rep=actr.String('j'))
    m.add_chunk('k', 'letter', letter='k', next='l', visual_rep=actr.String('k'), vocal_rep=actr.String('k'))
    m.add_chunk('l', 'letter', letter='l')
    m.add_dm('attending')
    m.add_dm('read')
    m.add_dm('count')
    m.add_dm('counting')
    m.add_dm('encode')

    m.set_visloc_default(screen_x='lowest')

    m.add_production(
        '''
        (p attend
           =goal>
              isa         goal
              state       nil
           =visual-location>
           ?visual>
               state      free
        ==>
           =goal>
              state       attending
           +visual>
              cmd         move-attention
              screen-pos  =visual-location
           )
        '''
    )

    m.add_production(
        '''
        (p read-first
           =goal>
             isa         goal
             state       attending
           =visual>
             isa         visual-object
             value       =char
           ?imaginal>
             buffer      empty
             state       free
        ==>
           +imaginal>
           +retrieval>
             isa         letter
             visual-rep  =char
           =goal>
             state       nil
           +visual-location>
             isa         visual-location
           > screen-x    current
             screen-x    lowest
           - value       "+"
           )
        '''
    )

    m.add_production(
        '''
        (p encode-first
           =goal>
             isa           goal
             state         attending
           =retrieval>
             letter        =let
             vocal-rep     =word
           =imaginal>
             arg1          nil
           ?vocal>
             preparation   free
           ==>
           +vocal>
             cmd           subvocalize
             string        =word
           =imaginal>
             arg1          =let
           =goal>
             state         read
           )
        '''
    )

    m.add_production(
        '''
        (p read-second
           =goal>
             isa         goal
             state       read
           =visual>
             isa         visual-object
             value       =char
           =imaginal>
             isa         problem
            - arg1       nil
             arg2        nil
           ==>
           =imaginal>
           +retrieval>
             isa         number
             visual-rep  =char

           =goal>
             state       nil

           +visual-location>
             isa         visual-location
             screen-x    highest
           )
        '''
    )

    m.add_production(
        '''
        (p encode-second
           =goal>
             isa          goal
             state        attending
           =retrieval>
             number       =num
             vocal-rep    =word
           =imaginal>
            - arg1        nil
             arg2         nil
           ?vocal>
             preparation  free
           ==>
           +vocal>
             cmd          subvocalize
             string       =word
           =imaginal>
             arg2         =num
           =goal>
             state        read
           )
        '''
    )

    m.add_production(
        '''
        (p read-third
           =goal>
             isa         goal
             state       read
           =imaginal>
             isa         problem
             arg1        =arg1
             arg2        =arg2
           =visual>
             isa         visual-object
             value       =char
           ?visual>
             state       free
        ==>
           =imaginal>
           +retrieval>
             isa         letter
             visual-rep  =char
           =goal>
             state       encode
           +visual>
             cmd         clear
           )
        '''
    )

    m.add_production(
        '''
        (p encode-third
           =goal>
             isa          goal
             state        encode
             target       nil
           =retrieval>
             letter       =let
             vocal-rep    =word
           =imaginal>
             arg1         =a1
             arg2         =a2
           ?vocal>
             preparation  free
           ==>
           +vocal>
             cmd          subvocalize
             string       =word
           =goal>
             target       =let
             state        count
           =imaginal>
           )
        '''
    )

    m.add_production(
        '''
        (p start-counting
           =goal>
             isa         goal
             state       count
           =imaginal>
             isa         problem
             arg1        =a
             arg2        =val
        ==>
           =imaginal>
             result      =a
           =goal>
             state       counting
           +retrieval>
             isa         letter
             letter      =a
           )
        '''
    )

    m.add_production(
        '''
        (p initialize-counting
           =goal>
             isa         goal
             state       counting
             count       nil
           =retrieval>
             isa         letter
             letter      =l
             next        =new
             vocal-rep   =t
           ?vocal>
             state       free
           ==>
           +vocal>
             isa         subvocalize
             string      =t
           =goal>
             next-num    one
             next-let    =new
             count       zero
           +retrieval>
             isa         letter
             letter      =new
           )
        '''
    )

    m.add_production(
        '''
        (p update-result
           =goal>
             isa         goal
             count       =val
             next-let    =let
             next-num    =n
           =imaginal>
             isa         problem
            - arg2       =val
           =retrieval>
             isa         letter
             letter      =let
             next        =new
             vocal-rep   =txt
           ?vocal>
             state       free
           ==>
           =goal>
             next-let    =new
           +vocal>
             cmd         subvocalize
             string      =txt
           =imaginal>
             result      =let
           +retrieval>
             isa         number
             number      =n
           )
        '''
    )

    m.add_production(
        '''
        (p update-count
           =goal>
             isa         goal
             next-let    =let
             next-num    =n
           =retrieval>
             isa         number
             number      =val
             next        =new
             vocal-rep   =txt
           ?vocal>
             preparation free
        ==>
           +vocal>
             cmd         subvocalize
             string      =txt
           =goal>
             count       =val
             next-num    =new
           +retrieval>
             isa         letter
             letter      =let
           )
        '''
    )

    m.add_production(
        '''
        (p final-answer-yes
           =goal>
             isa         goal
             target      =let
             count       =val
           =imaginal>
             isa         problem
             result      =let
             arg2        =val

           ?manual>
             state       free
           ==>
           +goal>

           +manual>
             cmd         press-key
             key         "k"
           )
        '''
    )

    m.add_production(
        '''
        (p final-answer-no
            =goal>
             isa         goal
             count       =val
           - target      =let
           =imaginal>
             isa         problem
             result      =let
             arg2        =val

           ?manual>
             state       free
           ==>
           +goal>

           +manual>
             cmd         press-key
             key         "d"
           )
        '''
    )

    m.set_all_base_levels(100000, -1000)

    m.goal_focus(isa='goal')
    return m


actr.clear_state()

# ACT-R tutorial unit 4 zbrodoff task.
# This experiment presents participants with alpha-arithmetic
# problems like "A + 2 = C" which they must respond to by
# pressing k if the problem is correct or d if it is not.
# The code runs the control condition from the paper:
#
# Zbrodoff, N. J. (1995).  Why is 9 + 7 harder than 2 + 3? 
# Strength and interference as explanations of the problem-size 
# effect.  Memory & Cognition, 23 (6), 689-700.
#
# That condition presents problems with addends of 2, 3, and 4 
# with equal frequency in blocks of 192 trials where half of the
# trials in a block are correct and half are false.  The 
# data for comparison is the average response time by block
# and addend for correct answers (including both true and
# false problems).

# Import the actr module for tutorial tasks


# Load the corresponding model for the task

MODEL = actr.use(build())

# Global variables to hold the trials to present, the results that have
# been collected, and the original data for comparison.

trials = []
results = []

control_data = [1.84, 2.46, 2.82, 1.21, 1.45, 1.42, 1.14, 1.21, 1.17]

# Also create a variable to indicate whether it will be a model or person.
# This is done to keep the number of parameters needed to run the functions
# smaller since one may want a visible window for either a person or model
# and this avoids having to specify both who is running and whether the window
# should be shown.

run_model = True

# Because the data collection for this task is a little more involved
# we're going to record the trials in a class to keep everything
# together and organized instead of just a list of items as has been
# done for other experiments.  This will also allow us to store all of
# the information needed to present a trial together so that we can 
# create them all in advance which can be useful when running in an
# event-driven style.  A trial will hold the block number, the addend
# value, the text of the problem to display, the correct answer, whether
# the window should be visible or not, whether the response from the
# participant was correct, the time the trial started, and the response
# time.

class trial():
    def __init__(self,block,addend1,addend2,sum,answer,visible=None):
        self.block = block
        self.addend2 = addend2
        self.text = addend1 + " + " + addend2 + " = " + sum
        self.answer = answer.lower()
        if visible == None:
            self.visible = not(run_model)
        else:
            self.visible = visible
        self.correct = False

# The present_trial function takes one parameter which is a trial
# structure and an optional parameter which indicates whether or not
# to open a new window for this trial (since this task is running 
# continuously it will run faster if it uses the same window repeatedly,
# but because the same code is used to run it for a variety of
# different situations it needs to know when to start over with a
# new display). 

def present_trial(trial, new_window = True):

    if new_window:
        # If a new window is requested it opens one using
        # the visible status indicated in the trial and
        # if the model is performing the task it installs
        # that window device for the model.

        w = actr.open_exp_window("Alpha-arithmetic Experiment", visible=trial.visible)
        if run_model:
          actr.install_device(w)
    else:

        # otherwise it just clears the current window

        actr.clear_exp_window()

    # add the text from the trial to the window and set the
    # start time in the trial structure.

    actr.add_text_to_exp_window(None, trial.text, x=100, y=150)

    trial.start = actr.get_time(run_model)


# The respond_to_key_press function will be set up to monitor
# the output-key actions, and thus will be called with two parameters
# when a key is pressed: the name of the model that pressed the key 
# (or None if it is a person) and the string naming the key that was
# pressed. 
# Unlike the previous tasks, since this one is event-driven we will
# actually do more than just record the key and time in this function.
# It will also present the next trial if there is one so that the
# model can continue to run in the task until it is complete.

def respond_to_key_press (model,key):
    global trials,results

    # Set the response time and correctness in the trial
    # and add it to the results list.

    trials[0].time = (actr.get_time(run_model) - trials[0].start) / 1000.0

    if key.lower() == trials[0].answer :
        trials[0].correct = True

    results.append(trials[0])

    # Remove the current trial, and if there are any trials left to 
    # present then present the first of them now.

    trials = trials[1:]

    if len(trials) > 0 :
        present_trial(trials[0],False)


# The collect_responses function takes no parameters and runs all of
# the trials available.

def collect_responses():

    # record how many trials need to be run

    total = len(trials)

    # Create a command for respond_to_key_press and monitor output-key.

    actr.add_command("zbrodoff-response", respond_to_key_press,
                     "Zbrodoff task key press response monitor")
    actr.monitor_command("output-key","zbrodoff-response")

    # present the first trial

    present_trial(trials[0])

    # If it's a model doing the task run for 10s per trial,
    # and if it's a person loop until there are as many results
    # as there were trials to run.

    if run_model :
        actr.run(10 * total)
    else:
        if actr.visible_virtuals_available():
            while len(results) < total:
                actr.process_events()

    # stop monitoring and remove the command

    actr.remove_command_monitor("output-key","zbrodoff-response")
    actr.remove_command("zbrodoff-response")


# The problem function takes 4 required parameters.
# The first three are the strings of the elements of the problem
# to present e.g. "A","2","C" to preset "A + 2 = C".  The fourth
# is a string with the key press that will be a correct response
# which is "k" if the problem is correct and "d" if the problem
# is not correct.  The optional parameter can be specified as True
# to have the window displayed, but if not provided defaults to
# not showing the window.
# It clears the current results, creates a list with the single
# trial specified and runs the task for that trial and displays
# the results.

def problem(addend1,addend2,sum,answer,visible=None):

    global results
    results = []

    global trials
    trials = [trial(1,addend1,addend2,sum,answer,visible)]

    collect_responses()
    return analyze_results()

# set and block are similar to problem except that instead 
# of presenting a single trial they present a full set (24 
# trials) or block (192 trials) of items.

def set(visible=None):

    global results
    results = []

    global trials
    trials = create_set(1,visible)

    collect_responses()
    return analyze_results()

def block(visible=None):

    global results
    results = []

    global trials
    trials = []

    for i in range(8):
        trials = trials + create_set(1,visible)

    collect_responses()
    return analyze_results()


# experiment has two optioal parameters.  The first is
# whether or not to show the window which defaults to not shown,
# and the second is whether or not to display the results after
# the experiment is run (the default is to show them).
# It resets the model, generates three blocks of trials, runs
# those trials, and reports the results.

def experiment(visible=None,show=True):

    actr.reset()

    global trials
    trials = []

    for j in range(3):
        for i in range(8):
            trials = trials + create_set(j+1,visible)

    global results
    results = []

    collect_responses()
    return analyze_results(show)


# compare takes one required parameter which is the number
# of times to run a model through the full experiment.  It runs
# the model that many times and averages the results of those
# runs which it compares to the original data for the task and
# then displays the results.

def compare(n):

    rts = [0,0,0,0,0,0,0,0,0]
    counts = [0,0,0,0,0,0,0,0,0]

    for i in range(n):
        r,c = experiment(False,False)
        rts = list(map(lambda x,y: x + y,rts,r))
        counts = list(map(lambda x,y: x + y,counts,c))
    
    rts = list(map(lambda x: x/n,rts))
    counts = list(map(lambda x: x/n,counts))

    actr.correlation(rts,control_data)
    actr.mean_deviation(rts,control_data)

    print_analysis(rts,counts,[1,2,3],['2','3','4'], [192,192,192])

# analyze_results takes one optional parameter which 
# indicates whether or not to print the results in addition
# to averaging the times by addend and block and returning
# the averaged results and counts of correct items in a list.

def analyze_results(show=True):

    blocks = []
    addends = []
    data = dict()    
    totals = dict()

    for i in results:
        if i.addend2 in totals:
            totals[i.addend2] += 1
        else:
            totals[i.addend2] = 1
 
        if i.correct:
            if (i.block,i.addend2) in data:
                data[(i.block,i.addend2)].append(i.time)
            else:
                data[(i.block,i.addend2)]=[i.time]
                if i.block not in blocks:
                    blocks.append(i.block)
                if i.addend2 not in addends:
                    addends.append(i.addend2)


    blocks.sort()
    addends.sort()

    rts =[]
    counts =[]
    for b in blocks:
        for a in addends:
            rts.append(sum(data[(b,a)]) / len(data[(b,a)]))
            counts.append(len(data[(b,a)]))

    if show:
        print_analysis(rts,counts,blocks,addends,[totals[i] for i in addends])

    return (rts, counts)
    
# print_analysis displays a table with the data items provided.

def print_analysis(rts,counts,blocks,addends,totals):

    print()
    print("         ", end="")
    for a,t in zip(addends,map(lambda x: x/len(blocks), totals)):
        print("%6s (%2d) " % (a, t),end="")
    print()
    for b in range(len(blocks)):
        print("Block  %d" % blocks[b],end="")
        for a in range(len(addends)):
            print(" %6.3f (%2d)" % (rts[a+b*len(addends)],counts[a+b*len(addends)]),end="")
        print()


# This varaible holds the problems to be presented
# in one set of the task -- 4 problems with each addend
# in a correct equation and 4 problems with each addend
# in an incorrect equation.

data_set = [["a","2","c","k"],["d","2","f","k"],
            ["b","3","e","k"],["e","3","h","k"],
            ["c","4","g","k"],["f","4","j","k"],
            ["a","2","d","d"],["d","2","g","d"],
            ["b","3","f","d"],["e","3","i","d"],
            ["c","4","h","d"],["f","4","k","d"],
            ["a","2","c","k"],["d","2","f","k"],
            ["b","3","e","k"],["e","3","h","k"],
            ["c","4","g","k"],["f","4","j","k"],
            ["a","2","d","d"],["d","2","g","d"],
            ["b","3","f","d"],["e","3","i","d"],
            ["c","4","h","d"],["f","4","k","d"]]


# create_set takes a block number and whether the items
# should be visible and returns a randomized list of
# trial structures representing one set of data with
# those conditions.
 
def create_set(block,visible):

    return list(map(lambda x: trial(block,*x,visible=visible), actr.permute_list(data_set)))


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
