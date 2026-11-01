"""Python port of ACT-R:tutorial;unit8;bst-ppm-model.lisp plus the bst_ppm task."""

from __future__ import annotations

import json
import sys

import rust_actr.actr as actr

DEFAULT_FUNCTION = 'experiment'
DEFAULT_ARGS = [1]


def build(seed=(100, 0)) -> actr.Model:
    m = actr.Model('bst-learn', seed=seed)

    m.sgp('v', None)
    m.sgp('esc', True)
    m.sgp('egs', 3)
    m.sgp('show_focus', True)
    m.sgp('trace_detail', 'medium')
    m.sgp('ul', True)
    m.sgp('ult', True)
    m.sgp('needs_mouse', True)
    m.sgp('ppm', 40)
    m.sgp('sim_hook', actr.String('bst-number-sims'))
    m.sgp('cache_sim_hook_results', True)

    m.chunk_type('try-strategy', 'strategy', 'state')
    m.chunk_type('encoding', 'a-loc', 'b-loc', 'c-loc', 'goal-loc', 'length', 'goal-length', 'b-len', 'c-len', 'difference')

    m.define_chunks('start')
    m.define_chunks('find-line')
    m.define_chunks('looking')
    m.define_chunks('attending')
    m.define_chunks('choose-strategy')
    m.define_chunks('consider-next')
    m.define_chunks('check-for-done')
    m.define_chunks('read-done')
    m.define_chunks('evaluate-c')
    m.define_chunks('prepare-mouse')
    m.define_chunks('evaluate-a')
    m.define_chunks('over')
    m.define_chunks('under')
    m.define_chunks('move-mouse')
    m.define_chunks('wait-for-click')

    m.goal_focus(isa='try-strategy', state='start')

    m.declare_buffer_usage('imaginal', 'encoding', 'difference', 'length')

    m.add_production(
        '''
        (p start-trial
            =goal>
              isa    try-strategy
              state  start
            ?visual-location>
              buffer unrequested
           ==>
            =goal>
              state  find-line)
        '''
    )

    m.add_production(
        '''
        (p find-next-line
            =goal>
              isa       try-strategy
              state     find-line
           ==>
            +visual-location>
              isa       visual-location
              :attended nil
              kind      line
              screen-y  lowest
            =goal>
              state     looking)
        '''
    )

    m.add_production(
        '''
        (p attend-line
            =goal>
              isa     try-strategy
              state   looking
            =visual-location>
            ?visual>
              state   free
           ==>
            =goal>
              state      attending
            +visual>
              cmd        move-attention
              screen-pos =visual-location)
        '''
    )

    m.add_production(
        '''
        (p encode-line-a
            =goal>
              isa        try-strategy
              state      attending
            =visual>
              isa        line
              screen-pos =pos
            ?imaginal>
              buffer     empty
              state      free
           ==>
            +imaginal>
              isa      encoding
              a-loc    =pos
            =goal>
              state    find-line)
        '''
    )

    m.add_production(
        '''
        (p encode-line-b
            =goal>
              isa        try-strategy
              state      attending
            =imaginal>
              isa        encoding
              a-loc      =a
              b-loc      nil
            =visual>
              isa        line
              screen-pos =pos
              width      =b-len
           ==>
            =imaginal>
              b-loc    =pos
              b-len    =b-len
            =goal>
              state    find-line)
        '''
    )

    m.add_production(
        '''
        (p encode-line-c
            =goal>
              isa        try-strategy
              state      attending
            =imaginal>
              isa        encoding
              b-loc      =b
              c-loc      nil
            =visual>
              isa        line
              screen-pos =pos
              width      =c-len
           ==>
            =imaginal>
              c-loc    =pos
              c-len    =c-len
            =goal>
              state    find-line)
        '''
    )

    m.add_production(
        '''
        (p encode-line-goal
            =goal>
              isa        try-strategy
              state      attending
            =imaginal>
              isa        encoding
              c-loc      =c
              goal-loc   nil
            =visual>
              isa        line
              screen-pos =pos
              width      =length
            ?visual>
              state      free
           ==>
            =imaginal>
              goal-loc     =pos
              goal-length  =length
            =goal>
              state        choose-strategy)
        '''
    )

    m.add_production(
        '''
        (p encode-line-current
            =goal>
              isa      try-strategy
              state    attending
            =imaginal>
              isa      encoding
              goal-loc =goal-loc
            =visual>
              isa      line
              width    =current-len
            ?visual>
              state    free
            ?imaginal-action>
              state    free
          ==>
            =imaginal>
              length     =current-len
            +imaginal-action>
              action     "bst-compute-difference"
              simple     t
            =goal>
              state      consider-next
            +visual>
              cmd        move-attention
              screen-pos =goal-loc)
        '''
    )

    m.add_production(
        '''
        (p check-for-done
            =goal>
              isa         try-strategy
              state       consider-next
            =imaginal>
              isa         encoding
             > difference -1
             < difference 1
            ?imaginal>
              state       free
           ==>
            =goal>
              state     check-for-done
            +visual-location>
              isa       visual-location
              value     "done")
        '''
    )

    m.add_production(
        '''
        (p find-done
            =goal>
              isa      try-strategy
              state    check-for-done
            =visual-location>
            ?visual>
              state    free
           ==>
            +visual>
              cmd        move-attention
              screen-pos =visual-location
            =goal>
              state      read-done)
        '''
    )

    m.add_production(
        '''
        (p read-done
            =goal>
              isa      try-strategy
              state    read-done
            =visual>
              isa      text
              value    "done"
        ==>
            +goal>
              isa      try-strategy
              state    start)
        '''
    )

    m.add_production(
        '''
        (p consider-c
            =goal>
              isa         try-strategy
              state       consider-next
            =imaginal>
              isa         encoding
              c-loc       =c-loc
             - difference 0
            ?imaginal>
             state        free
            ?visual>
               state      free
           ==>
            =imaginal>
            =goal>
              state      evaluate-c
            +visual>
              cmd        move-attention
              screen-pos =c-loc)
        '''
    )

    m.add_production(
        '''
        (p choose-c
            =goal>
              isa        try-strategy
              state      evaluate-c
            =imaginal>
              isa        encoding
              difference =difference
            =visual>
              isa        line
            <= width     =difference
           ==>
            =imaginal>
            =goal>
              state    prepare-mouse
            +visual-location>
              isa      visual-location
              kind     oval
              value    "c")
        '''
    )

    m.add_production(
        '''
        (p consider-a
            =goal>
              isa        try-strategy
              state      evaluate-c
            =imaginal>
              isa        encoding
              a-loc      =a-loc
              difference =difference
            =visual>
              isa        line
            > width      =difference
            ?visual>
              state free
           ==>
            =imaginal>
            =goal>
              state      evaluate-a
            +visual>
              cmd        move-attention
              screen-pos =a-loc)
        '''
    )

    m.add_production(
        '''
        (p choose-a
            =goal>
              isa        try-strategy
              state      evaluate-a
            =imaginal>
              isa        encoding
              difference =difference
            =visual>
              isa        line
            <= width     =difference
           ==>
            =imaginal>
            =goal>
              state    prepare-mouse
            +visual-location>
              isa      visual-location
              kind     oval
              value    "a")
        '''
    )

    m.add_production(
        '''
        (p reset
            =goal>
              isa        try-strategy
              state      evaluate-a
            =imaginal>
              isa        encoding
              difference =difference
            =visual>
              isa        line
             > width     =difference
           ==>
            =imaginal>
            =goal>
              state    prepare-mouse
            +visual-location>
              isa      visual-location
              kind     oval
              value    "reset")
        '''
    )

    m.add_production(
        '''
        (p decide-over
            =goal>
              isa         try-strategy
              state       choose-strategy
             - strategy   over
            =imaginal>
              isa         encoding
              goal-length =d
              b-len       =d
           ==>
            =imaginal>
            =goal>
              state    prepare-mouse
              strategy over
            +visual-location>
              isa      visual-location
              kind     oval
              value    "b")
        '''
    )

    m.add_production(
        '''
        (p decide-under
            =goal>
              isa         try-strategy
              state       choose-strategy
            -  strategy   under
            =imaginal>
              isa         encoding
              goal-length =d
              c-len       =d
           ==>
            =imaginal>
            =goal>
              state    prepare-mouse
              strategy under
            +visual-location>
              isa      visual-location
              kind     oval
              value    "c")
        '''
    )

    m.add_production(
        '''
        (p move-mouse
            =goal>
              isa    try-strategy
              state  prepare-mouse
            =visual-location>
            ?visual>
              state  free
            ?manual>
              state  free
           ==>
            +visual>
              cmd        move-attention
              screen-pos =visual-location
            =goal>
              state      move-mouse
            +manual>
              cmd        move-cursor
              loc        =visual-location)
        '''
    )

    m.add_production(
        '''
        (p click-mouse
            =goal>
              isa     try-strategy
              state   move-mouse
            ?manual>
              state   free
           ==>
            =goal>
              state   wait-for-click
            +manual>
              cmd     click-mouse)
        '''
    )

    m.add_production(
        '''
        (p look-for-current
            =goal>
              isa       try-strategy
              state     wait-for-click
            ?manual>
              state     free
            =visual>
             - value "reset"
           ==>
            +visual-location>
              isa       visual-location
              kind      line
              screen-y  highest
            =goal>
              state     looking)
        '''
    )

    m.add_production(
        '''
        (p pick-another-strategy
            =goal>
              isa       try-strategy
              state     wait-for-click
            ?manual>
              state     free
            =visual>
              value     "reset"
           ==>
            =goal>
              state choose-strategy)
        '''
    )
    m.spp('decide-over', **{'u': 10})
    m.spp('decide-under', **{'u': 10})
    m.spp('pick-another-strategy', **{'reward': 0})
    m.spp('read-done', **{'reward': 20})
    return m


actr.clear_state()

# ACT-R tutorial unit6 building sticks experiment.
# This experiment displays three posssible sticks
# which can be used to create a given target stick's
# length.  It is an isomorph of Luchins water jug
# problem, and the experiment for the model is the
# one from: 
#
# Lovett, M. C., & Anderson, J. R. (1996).  History of success 
# and current context in problem solving: Combined influences
# on operator selection.  Cognitive Psychology, 31, 168-217.
#
# The task is presented with buttons to pick the sticks
# and a button to reset the current trial.

# Import the actr module for tutorial tasks and the numbers
# module for the Number class

import numbers

# Global variables to hold the information about the
# current trial information.

target = None
current_stick = None
current_line = None
done = False
choice = None
window = None
visible = False

# The data from the experiment, the lengths of the sticks
# used in the experiment, and two example problems for 
# demonstration.

exp_data = [20, 67, 20, 47, 87, 20, 80, 93, 83, 13, 29, 27, 80, 73, 53]
exp_stims = [[15,250,55,125],[10,155,22,101],[14,200,37,112],
             [22,200,32,114],[10,243,37,159],[22,175,40,73],
             [15,250,49,137],[10,179,32,105],[20,213,42,104],
             [14,237,51,116],[12,149,30,72],[14,237,51,121],
             [22,200,32,114],[14,200,37,112],[15,250,55,125]]

no_learn_stims = [[15,200,41,103],[10,200,29,132]]

# build_display takes the lengths of the sticks for a trial.
# It sets the global variables and draws the initial interface.

def build_display (a,b,c,goal):
    global window,target,current_stick,done,current_line,choice

    target = goal
    current_stick = 0
    done = False
    choice = None
    current_line = None
    window = actr.open_exp_window("Building Sticks Task",visible=visible,width=600,height=400)

    actr.add_button_to_exp_window(window, text="A", x=5, y=23, action=["bst-ppm-button-pressed",a,"under"], height=24, width=40)
    actr.add_button_to_exp_window(window, text="B", x=5, y=48, action=["bst-ppm-button-pressed",b,"over"], height=24, width=40)
    actr.add_button_to_exp_window(window, text="C", x=5, y=73, action=["bst-ppm-button-pressed",c,"under"], height=24, width=40)
    actr.add_button_to_exp_window(window, text="Reset", x=5, y=123, action="bst-ppm-reset-button-pressed", height=24, width=65)

    actr.add_line_to_exp_window(window,[75,35],[a + 75,35],"black")
    actr.add_line_to_exp_window(window,[75,60],[b + 75,60],"black")
    actr.add_line_to_exp_window(window,[75,85],[c + 75,85],"black")
    actr.add_line_to_exp_window(window,[75,110],[goal + 75,110],"green")

# button_pressed will be added as the bst-button-pressed command
# for use as the action of the stick choice buttons.  It takes
# a parameter to indicate the length of the stick and whether
# the stick is associated with under or over shoot as a first
# choice.   

def button_pressed(len,dir):
    global choice,current_stick

    if not(choice):
        choice = dir

    if not(done):
        if current_stick > target:
            current_stick -= len
        else:
            current_stick += len

        update_current_line()

# reset_display will be added as the bst-reseet-button-pressed 
# command for use as the action of the reset buttons.  If the
# trial is not over, then it sets the current stick length to 0
# and redraws it.
    
def reset_display():
    global current_stick

    if not(done):
        current_stick = 0
        update_current_line()

# Add the commands for those two functions so they can be
# used as button actions.

actr.add_command("bst-ppm-button-pressed",button_pressed,"Choice button action for the Building Sticks Task.  Do not call directly")
actr.add_command("bst-ppm-reset-button-pressed",reset_display,"Reset button action for the Building Sticks Task.  Do not call directly")

# update_current_line compares the length of the current
# stick to the target stick length.  If they match the
# the trial is over, it redraws the current line, and 
# displays the done prompt.  If it is zero it removes the
# line from the display.  If there is a current line then
# it is updated to match the current length, and if there
# is not a current line then one is drawn and saved for
# future modification.

def update_current_line():
    global current_line,done

    if current_stick == target:
        done = True
        actr.modify_line_for_exp_window(current_line, [75,135], [target + 75,135])
        actr.add_text_to_exp_window(window, "Done", x=180, y=200)
    elif current_stick == 0:
        if current_line:
            actr.remove_items_from_exp_window(window,current_line)
            current_line = None
    elif current_line:
        actr.modify_line_for_exp_window(current_line,[75,135],[current_stick + 75,135])
    else:
        current_line = actr.add_line_to_exp_window(window,[75,135],[current_stick + 75,135],"blue")

# do_experiment takes a required parameter which is
# a list of stick lengths and an optional parameter
# which indicates whether a person is doing the task.
# It draws the initial sticks and then waits for
# a person to complete the task or runs the model
# for up to a minute to do the task.

def do_experiment(sticks, human=False):
    build_display(*sticks)
  
    if human:
        if actr.visible_virtuals_available():
            wait_for_human()
    else:
        actr.install_device(window)
        actr.start_hand_at_mouse()
        actr.run(60,visible)

# wait_for_human takes no parameters. It waits for
# a person to finish the task, and then waits one 
# more second after the done prompt is displayed to
# give the person a chance to read it.

def wait_for_human ():
    while not(done):
        actr.process_events()

    start = actr.get_time(False)
    while (actr.get_time(False) - start) < 1000:
        actr.process_events()

# bst_set takes three required parameters and one optional
# parameter.  The first parameter indicates whether it 
# is a person or the model performing the task, and the
# second indicates whether it should use a visible or
# virtual window.  The third parameter is a list of 
# stick lengths for the trials to present.  The optional
# parameter indicates whether the model should learn from
# trial to trial or be reset before each new trial.
# It returns a list of strings indicating whether each
# trial presented was started with the over-shoot or
# under-shoot approach.

def bst_set(human,vis,stims,learn=True):
    global visible

    result = []

    visible = vis

    for stim in stims:
        if not(learn) and not(human):
            actr.reset()
        do_experiment(stim,human)
        result.append(choice)

    return result

# test is used to run multiple instances of the 2 demo
# problems.  It takes one required parameter which indicates
# how many times to run that set of two items, and an optional
# parameter to indicate if it should be a person or model
# doing the task.  It returns a list with the counts of the
# times over-shoot was tried on each of the problems.
# When the model runs the task it is not learning, and starts
# each trial as if it were the first time doing the task.
# If the model is running once through the set then it will
# use a visible window to show the interaction, otherwise it
# will use a virtual window.

def test(n,human=False):
    
    l = len(no_learn_stims)

    result = [0]*l

    if human or (n == 1):
        v = True
    else:
        v = False

    for i in range(n):

        d = bst_set(human,v,no_learn_stims,False)

        for j in range(l):
            if d[j] == "over":
                result[j] += 1

    return result


# experiment is used to run the full experiment multiple
# times and report the results and fit to the experiment data.
# It has a required parameter which indicates how many times
# to run the task, and an optional parameter indicating whether
# it should be a person performing the task.
# It collects the over- or under- shoot choices for each problem
# and computes the proportion of time it's chosen for comparison
# to the original data.  It displays the data and its fit to the
# data from the original experiment along with the average utility
# value over the trials for each of the four productions in the 
# model which make the choice.

def experiment(n,human=False):

    l = len(exp_stims)

    result = [0] * l
    p_values = [["decide-over",0],["decide-under",0]]
    for i in range(n):
        actr.reset()
        d = bst_set(human,human,exp_stims)
        for j in range(l):
            if d[j] == "over":
                result[j] += 1

        for p in p_values:
            p[1]+=production_u_value(p[0])

    result = list(map(lambda x: 100 * x / n,result))

    if len(result) == len(exp_data):
        actr.correlation(result,exp_data)
        actr.mean_deviation(result,exp_data)

    print()
    print("Trial ",end="")
    for i in range(l):
        print("%-8d"%(i + 1),end="")
    print()

    print("  ",end="")
    for i in range(l):
        print("%8.2f"%result[i],end="")

    print()
    print()

    for p in p_values:
        print("%-12s: %6.4f"%(p[0],p[1]/n))

# production_u_value returns the current :u parameter
# value from the indicated production. 
# It calls the ACT-R spp command using the no_output 
# function to temporarily disabled the command output to
# keep the spp command from printing the information.

def production_u_value(prod):
    return actr.no_output("spp",prod,":u")[0][0]

# A similarity hook function to return
# similarity values between numbers which
# are expected to be line lengths.

def number_sims(a,b):
    if isinstance(b,numbers.Number) and isinstance(a,numbers.Number):
        return  abs(a - b) / -300
    else:
        return False    

actr.add_command("bst-number-sims",number_sims,"Similarity hook function for building sticks task.")

# The compute_difference function is used as the
# function called by an imaginal-action buffer
# request.  It creates a chunk which is a copy of
# the chunk in the imaginal buffer and adds a
# slot called difference which holds the difference
# between the length of the current line and the
# target line.  That chunk's name is returned
# so that the imaginal module will place it into
# the imaginal buffer.

def compute_difference():
    c = actr.buffer_read('imaginal')
    n = actr.copy_chunk(c)
    actr.mod_chunk(n,'difference',abs(actr.chunk_slot_value(c,'length') - actr.chunk_slot_value(c,'goal-length')))
    return n

actr.add_command("bst-compute-difference",compute_difference,"Imaginal action function to compute the difference between sticks.")


# Load the corresponding ACT-R starting model.
# Done after adding the bst-compute-difference and bst-number-sims
# commands because they are used in the model and should exist first.

MODEL = actr.use(build())


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
