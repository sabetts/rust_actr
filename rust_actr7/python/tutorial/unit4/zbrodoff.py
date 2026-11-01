"""Python port of ACT-R:tutorial;unit4;zbrodoff-model.lisp plus the zbrodoff task."""

from __future__ import annotations

import json
import sys

import rust_actr.actr as actr

LOGICAL_PATH = 'ACT-R:tutorial;unit4;zbrodoff-model.lisp'
MODEL_NAME = 'ZBRODOFF'
DEFAULT_FUNCTION = 'experiment'
DEFAULT_ARGS = []


def build(seed=(100, 0)) -> actr.Model:
    m = actr.Model('ZBRODOFF', seed=seed)
    m.sgp(':V', None)
    m.sgp(':ESC', True)
    m.sgp(':LF', 0.4)
    m.sgp(':BLL', 0.5)
    m.sgp(':ANS', 0.5)
    m.sgp(':RT', 0)
    m.sgp(':NCNAR', None)
    m.sgp(':HEAR-SELF', None)
    m.sgp(':SHOW-FOCUS', True)
    m.chunk_type('PROBLEM', 'ARG1', 'ARG2', 'RESULT')
    m.chunk_type('GOAL', 'STATE', 'COUNT', 'TARGET', 'NEXT-LET', 'NEXT-NUM')
    m.chunk_type('NUMBER', 'NUMBER', 'NEXT', 'VISUAL-REP', 'VOCAL-REP')
    m.chunk_type('LETTER', 'LETTER', 'NEXT', 'VISUAL-REP', 'VOCAL-REP')
    m.add_chunk('ZERO', 'NUMBER', **{'NUMBER': 'ZERO', 'NEXT': 'ONE', 'VISUAL-REP': actr.String('0'), 'VOCAL-REP': actr.String('zero')})
    m.add_chunk('ONE', 'NUMBER', **{'NUMBER': 'ONE', 'NEXT': 'TWO', 'VISUAL-REP': actr.String('1'), 'VOCAL-REP': actr.String('one')})
    m.add_chunk('TWO', 'NUMBER', **{'NUMBER': 'TWO', 'NEXT': 'THREE', 'VISUAL-REP': actr.String('2'), 'VOCAL-REP': actr.String('two')})
    m.add_chunk('THREE', 'NUMBER', **{'NUMBER': 'THREE', 'NEXT': 'FOUR', 'VISUAL-REP': actr.String('3'), 'VOCAL-REP': actr.String('three')})
    m.add_chunk('FOUR', 'NUMBER', **{'NUMBER': 'FOUR', 'NEXT': 'FIVE', 'VISUAL-REP': actr.String('4'), 'VOCAL-REP': actr.String('four')})
    m.add_chunk('FIVE', 'NUMBER', **{'NUMBER': 'FIVE'})
    m.add_chunk('A', 'LETTER', **{'LETTER': 'A', 'NEXT': 'B', 'VISUAL-REP': actr.String('a'), 'VOCAL-REP': actr.String('a')})
    m.add_chunk('B', 'LETTER', **{'LETTER': 'B', 'NEXT': 'C', 'VISUAL-REP': actr.String('b'), 'VOCAL-REP': actr.String('b')})
    m.add_chunk('C', 'LETTER', **{'LETTER': 'C', 'NEXT': 'D', 'VISUAL-REP': actr.String('c'), 'VOCAL-REP': actr.String('c')})
    m.add_chunk('D', 'LETTER', **{'LETTER': 'D', 'NEXT': 'E', 'VISUAL-REP': actr.String('d'), 'VOCAL-REP': actr.String('d')})
    m.add_chunk('E', 'LETTER', **{'LETTER': 'E', 'NEXT': 'F', 'VISUAL-REP': actr.String('e'), 'VOCAL-REP': actr.String('e')})
    m.add_chunk('F', 'LETTER', **{'LETTER': 'F', 'NEXT': 'G', 'VISUAL-REP': actr.String('f'), 'VOCAL-REP': actr.String('f')})
    m.add_chunk('G', 'LETTER', **{'LETTER': 'G', 'NEXT': 'H', 'VISUAL-REP': actr.String('g'), 'VOCAL-REP': actr.String('g')})
    m.add_chunk('H', 'LETTER', **{'LETTER': 'H', 'NEXT': 'I', 'VISUAL-REP': actr.String('h'), 'VOCAL-REP': actr.String('h')})
    m.add_chunk('I', 'LETTER', **{'LETTER': 'I', 'NEXT': 'J', 'VISUAL-REP': actr.String('i'), 'VOCAL-REP': actr.String('i')})
    m.add_chunk('J', 'LETTER', **{'LETTER': 'J', 'NEXT': 'K', 'VISUAL-REP': actr.String('j'), 'VOCAL-REP': actr.String('j')})
    m.add_chunk('K', 'LETTER', **{'LETTER': 'K', 'NEXT': 'L', 'VISUAL-REP': actr.String('k'), 'VOCAL-REP': actr.String('k')})
    m.add_chunk('L', 'LETTER', **{'LETTER': 'L'})
    m.add_dm('ATTENDING')
    m.add_dm('READ')
    m.add_dm('COUNT')
    m.add_dm('COUNTING')
    m.add_dm('ENCODE')
    m.set_visloc_default(SCREEN_X='LOWEST')
    m.add_production(
        '''(P ATTEND =GOAL> ISA GOAL STATE NIL =VISUAL-LOCATION> ?VISUAL> STATE FREE ==> =GOAL> STATE ATTENDING +VISUAL> CMD MOVE-ATTENTION SCREEN-POS =VISUAL-LOCATION)'''
    )
    m.add_production(
        '''(P READ-FIRST =GOAL> ISA GOAL STATE ATTENDING =VISUAL> ISA VISUAL-OBJECT VALUE =CHAR ?IMAGINAL> BUFFER EMPTY STATE FREE ==> +IMAGINAL> +RETRIEVAL> ISA LETTER VISUAL-REP =CHAR =GOAL> STATE NIL +VISUAL-LOCATION> ISA VISUAL-LOCATION > SCREEN-X CURRENT SCREEN-X LOWEST - VALUE "+")'''
    )
    m.add_production(
        '''(P ENCODE-FIRST =GOAL> ISA GOAL STATE ATTENDING =RETRIEVAL> LETTER =LET VOCAL-REP =WORD =IMAGINAL> ARG1 NIL ?VOCAL> PREPARATION FREE ==> +VOCAL> CMD SUBVOCALIZE STRING =WORD =IMAGINAL> ARG1 =LET =GOAL> STATE READ)'''
    )
    m.add_production(
        '''(P READ-SECOND =GOAL> ISA GOAL STATE READ =VISUAL> ISA VISUAL-OBJECT VALUE =CHAR =IMAGINAL> ISA PROBLEM - ARG1 NIL ARG2 NIL ==> =IMAGINAL> +RETRIEVAL> ISA NUMBER VISUAL-REP =CHAR =GOAL> STATE NIL +VISUAL-LOCATION> ISA VISUAL-LOCATION SCREEN-X HIGHEST)'''
    )
    m.add_production(
        '''(P ENCODE-SECOND =GOAL> ISA GOAL STATE ATTENDING =RETRIEVAL> NUMBER =NUM VOCAL-REP =WORD =IMAGINAL> - ARG1 NIL ARG2 NIL ?VOCAL> PREPARATION FREE ==> +VOCAL> CMD SUBVOCALIZE STRING =WORD =IMAGINAL> ARG2 =NUM =GOAL> STATE READ)'''
    )
    m.add_production(
        '''(P READ-THIRD =GOAL> ISA GOAL STATE READ =IMAGINAL> ISA PROBLEM ARG1 =ARG1 ARG2 =ARG2 =VISUAL> ISA VISUAL-OBJECT VALUE =CHAR ?VISUAL> STATE FREE ==> =IMAGINAL> +RETRIEVAL> ISA LETTER VISUAL-REP =CHAR =GOAL> STATE ENCODE +VISUAL> CMD CLEAR)'''
    )
    m.add_production(
        '''(P ENCODE-THIRD =GOAL> ISA GOAL STATE ENCODE TARGET NIL =RETRIEVAL> LETTER =LET VOCAL-REP =WORD =IMAGINAL> ARG1 =A1 ARG2 =A2 ?VOCAL> PREPARATION FREE ==> +VOCAL> CMD SUBVOCALIZE STRING =WORD =GOAL> TARGET =LET STATE COUNT =IMAGINAL>)'''
    )
    m.add_production(
        '''(P START-COUNTING =GOAL> ISA GOAL STATE COUNT =IMAGINAL> ISA PROBLEM ARG1 =A ARG2 =VAL ==> =IMAGINAL> RESULT =A =GOAL> STATE COUNTING +RETRIEVAL> ISA LETTER LETTER =A)'''
    )
    m.add_production(
        '''(P INITIALIZE-COUNTING =GOAL> ISA GOAL STATE COUNTING COUNT NIL =RETRIEVAL> ISA LETTER LETTER =L NEXT =NEW VOCAL-REP =T ?VOCAL> STATE FREE ==> +VOCAL> ISA SUBVOCALIZE STRING =T =GOAL> NEXT-NUM ONE NEXT-LET =NEW COUNT ZERO +RETRIEVAL> ISA LETTER LETTER =NEW)'''
    )
    m.add_production(
        '''(P UPDATE-RESULT =GOAL> ISA GOAL COUNT =VAL NEXT-LET =LET NEXT-NUM =N =IMAGINAL> ISA PROBLEM - ARG2 =VAL =RETRIEVAL> ISA LETTER LETTER =LET NEXT =NEW VOCAL-REP =TXT ?VOCAL> STATE FREE ==> =GOAL> NEXT-LET =NEW +VOCAL> CMD SUBVOCALIZE STRING =TXT =IMAGINAL> RESULT =LET +RETRIEVAL> ISA NUMBER NUMBER =N)'''
    )
    m.add_production(
        '''(P UPDATE-COUNT =GOAL> ISA GOAL NEXT-LET =LET NEXT-NUM =N =RETRIEVAL> ISA NUMBER NUMBER =VAL NEXT =NEW VOCAL-REP =TXT ?VOCAL> PREPARATION FREE ==> +VOCAL> CMD SUBVOCALIZE STRING =TXT =GOAL> COUNT =VAL NEXT-NUM =NEW +RETRIEVAL> ISA LETTER LETTER =LET)'''
    )
    m.add_production(
        '''(P FINAL-ANSWER-YES =GOAL> ISA GOAL TARGET =LET COUNT =VAL =IMAGINAL> ISA PROBLEM RESULT =LET ARG2 =VAL ?MANUAL> STATE FREE ==> +GOAL> +MANUAL> CMD PRESS-KEY KEY "k")'''
    )
    m.add_production(
        '''(P FINAL-ANSWER-NO =GOAL> ISA GOAL COUNT =VAL - TARGET =LET =IMAGINAL> ISA PROBLEM RESULT =LET ARG2 =VAL ?MANUAL> STATE FREE ==> +GOAL> +MANUAL> CMD PRESS-KEY KEY "d")'''
    )
    m.set_all_base_levels(100000, -1000)
    m.goal_focus(isa='GOAL', **{})
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
