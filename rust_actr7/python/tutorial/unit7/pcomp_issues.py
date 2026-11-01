"""Python port of ACT-R:tutorial;unit7;production-compilation-issues-model.lisp plus the pcomp_issues task."""

from __future__ import annotations

import json
import sys

import rust_actr.actr as actr

LOGICAL_PATH = 'ACT-R:tutorial;unit7;production-compilation-issues-model.lisp'
MODEL_NAME = 'COMPILATION-TEST'
DEFAULT_FUNCTION = 'game'
DEFAULT_ARGS = [1]


def build(seed=(100, 0)) -> actr.Model:
    m = actr.Model('COMPILATION-TEST', seed=seed)
    m.sgp(':ESC', True)
    m.sgp(':LF', 0.5)
    m.sgp(':BLL', 0.5)
    m.sgp(':MP', 18)
    m.sgp(':RT', -3)
    m.sgp(':ANS', 0.25)
    m.sgp(':STYLE-WARNINGS', None)
    m.sgp(':V', None)
    m.chunk_type('TASK', 'STATE')
    m.chunk_type('NUMBER', 'VISUAL-REP')
    m.chunk_type('RESPONSE', 'KEY', 'IS-RESPONSE', defaults={'IS_RESPONSE': True})
    m.chunk_type('TRIAL', 'NUM1', 'NUM2', 'RESULT', 'RESPONSE')
    m.define_chunks('WIN')
    m.define_chunks('DRAW')
    m.define_chunks('LOSE')
    m.define_chunks('ATTEND-NUM-1')
    m.define_chunks('ENCODE-NUM-1')
    m.define_chunks('FIND-NUM-2')
    m.define_chunks('ATTEND-NUM-2')
    m.define_chunks('ENCODE-NUM-2')
    m.define_chunks('NUM2')
    m.define_chunks('RETRIEVE-PAST-TRIAL')
    m.define_chunks('PROCESS-PAST-TRIAL')
    m.define_chunks('RESPOND')
    m.define_chunks('GUESS-OTHER')
    m.define_chunks('DETECT-FEEDBACK')
    m.define_chunks('ENCODE-FEEDBACK')
    m.add_chunk('ZERO', 'NUMBER', **{'VISUAL-REP': actr.String('0')})
    m.add_chunk('ONE', 'NUMBER', **{'VISUAL-REP': actr.String('1')})
    m.add_chunk('TWO', 'NUMBER', **{'VISUAL-REP': actr.String('2')})
    m.add_chunk('THREE', 'NUMBER', **{'VISUAL-REP': actr.String('3')})
    m.add_chunk('RESPONSE-1', 'RESPONSE', **{'KEY': actr.String('s')})
    m.add_chunk('RESPONSE-2', 'RESPONSE', **{'KEY': actr.String('d')})
    m.add_chunk('RESPONSE-3', 'RESPONSE', **{'KEY': actr.String('f')})
    m.set_similarities([('WIN', 'DRAW', -0.2), ('WIN', 'LOSE', -0.15), ('ZERO', 'ONE', -0.1), ('ONE', 'TWO', -0.1), ('TWO', 'THREE', -0.1), ('ZERO', 'TWO', -0.2), ('ONE', 'THREE', -0.2), ('ZERO', 'THREE', -0.3)])
    m.set_all_base_levels(1000, -100)
    m.goal_focus(isa='TASK', **{})
    m.add_production(
        '''(P DETECT-TRIAL-START =GOAL> ISA TASK STATE NIL =VISUAL-LOCATION> ISA VISUAL-LOCATION ?VISUAL> STATE FREE ?IMAGINAL> STATE FREE ==> =GOAL> STATE ATTEND-NUM-1 +IMAGINAL> ISA TRIAL +VISUAL> CMD MOVE-ATTENTION SCREEN-POS =VISUAL-LOCATION)'''
    )
    m.add_production(
        '''(P ATTEND-NUM-1 =GOAL> ISA TASK STATE ATTEND-NUM-1 =VISUAL> ISA VISUAL-OBJECT VALUE =VAL ==> =GOAL> STATE ENCODE-NUM-1 +RETRIEVAL> ISA NUMBER VISUAL-REP =VAL)'''
    )
    m.add_production(
        '''(P ENCODE-NUM-1 =GOAL> ISA TASK STATE ENCODE-NUM-1 =RETRIEVAL> ISA NUMBER =IMAGINAL> ISA TRIAL ==> =IMAGINAL> NUM1 =RETRIEVAL =GOAL> STATE FIND-NUM-2 +VISUAL-LOCATION> ISA VISUAL-LOCATION > SCREEN-X CURRENT :ATTENDED NIL)'''
    )
    m.add_production(
        '''(P FIND-NUM-2 =GOAL> ISA TASK STATE FIND-NUM-2 =VISUAL-LOCATION> ISA VISUAL-LOCATION ?VISUAL> STATE FREE ==> =GOAL> STATE ATTEND-NUM-2 +VISUAL> CMD MOVE-ATTENTION SCREEN-POS =VISUAL-LOCATION)'''
    )
    m.add_production(
        '''(P ATTEND-NUM-2 =GOAL> ISA TASK STATE ATTEND-NUM-2 =VISUAL> ISA VISUAL-OBJECT VALUE =VAL ==> =GOAL> STATE ENCODE-NUM-2 +RETRIEVAL> ISA NUMBER VISUAL-REP =VAL)'''
    )
    m.add_production(
        '''(P ENCODE-NUM-2 =GOAL> ISA TASK STATE ENCODE-NUM-2 =RETRIEVAL> ISA NUMBER =IMAGINAL> ISA TRIAL ==> =IMAGINAL> NUM2 =RETRIEVAL =GOAL> STATE RETRIEVE-PAST-TRIAL)'''
    )
    m.add_production(
        '''(P RETRIEVE-PAST-TRIAL =GOAL> ISA TASK STATE RETRIEVE-PAST-TRIAL =IMAGINAL> ISA TRIAL NUM1 =N1 NUM2 =N2 ==> =IMAGINAL> +RETRIEVAL> ISA TRIAL NUM1 =N1 NUM2 =N2 RESULT WIN =GOAL> STATE PROCESS-PAST-TRIAL)'''
    )
    m.add_production(
        '''(P NO-PAST-TRIAL =GOAL> ISA TASK STATE PROCESS-PAST-TRIAL ?RETRIEVAL> BUFFER FAILURE ==> +RETRIEVAL> ISA RESPONSE =GOAL> STATE RESPOND)'''
    )
    m.add_production(
        '''(P RETRIEVED-A-WIN =GOAL> ISA TASK STATE PROCESS-PAST-TRIAL =RETRIEVAL> ISA TRIAL RESULT WIN RESPONSE =RESPONSE ==> +RETRIEVAL> =RESPONSE =GOAL> STATE RESPOND)'''
    )
    m.add_production(
        '''(P RETRIEVED-A-NON-WIN =GOAL> ISA TASK STATE PROCESS-PAST-TRIAL =RETRIEVAL> ISA TRIAL - RESULT WIN RESPONSE =RESPONSE ==> +RETRIEVAL> =RESPONSE =GOAL> STATE GUESS-OTHER)'''
    )
    m.add_production(
        '''(P GUESS-OTHER =GOAL> ISA TASK STATE GUESS-OTHER =RETRIEVAL> ISA RESPONSE KEY =KEY ==> +RETRIEVAL> ISA RESPONSE - KEY =KEY =GOAL> STATE RESPOND)'''
    )
    m.add_production(
        '''(P RESPOND =GOAL> ISA TASK STATE RESPOND =RETRIEVAL> ISA RESPONSE KEY =KEY ?MANUAL> STATE FREE =IMAGINAL> ISA TRIAL ==> =IMAGINAL> RESPONSE =RETRIEVAL +MANUAL> CMD PRESS-KEY KEY =KEY =GOAL> STATE DETECT-FEEDBACK)'''
    )
    m.add_production(
        '''(P RESPOND-WHEN-RESPONSE-FAILURE =GOAL> ISA TASK STATE RESPOND ?RETRIEVAL> BUFFER FAILURE ?MANUAL> STATE FREE =IMAGINAL> ISA TRIAL ==> =IMAGINAL> RESPONSE RESPONSE-2 +MANUAL> CMD PRESS-KEY KEY "d" =GOAL> STATE DETECT-FEEDBACK)'''
    )
    m.add_production(
        '''(P DETECT-FEEDBACK =GOAL> ISA TASK STATE DETECT-FEEDBACK =VISUAL-LOCATION> ISA VISUAL-LOCATION ?VISUAL> STATE FREE ==> +VISUAL> CMD MOVE-ATTENTION SCREEN-POS =VISUAL-LOCATION =GOAL> STATE ENCODE-FEEDBACK)'''
    )
    m.add_production(
        '''(P ENCODE-FEEDBACK-WIN =GOAL> ISA TASK STATE ENCODE-FEEDBACK =IMAGINAL> ISA TRIAL =VISUAL> ISA VISUAL-OBJECT VALUE "win" ?MANUAL> STATE FREE ==> =IMAGINAL> RESULT WIN +MANUAL> CMD PRESS-KEY KEY SPACE +GOAL> ISA TASK)'''
    )
    m.add_production(
        '''(P ENCODE-FEEDBACK-LOSE =GOAL> ISA TASK STATE ENCODE-FEEDBACK =IMAGINAL> ISA TRIAL =VISUAL> ISA VISUAL-OBJECT VALUE "lose" ?MANUAL> STATE FREE ==> =IMAGINAL> RESULT LOSE +MANUAL> CMD PRESS-KEY KEY SPACE +GOAL> ISA TASK)'''
    )
    m.add_production(
        '''(P ENCODE-FEEDBACK-DRAW =GOAL> ISA TASK STATE ENCODE-FEEDBACK =IMAGINAL> ISA TRIAL =VISUAL> ISA VISUAL-OBJECT VALUE "draw" ?MANUAL> STATE FREE ==> =IMAGINAL> RESULT DRAW +MANUAL> CMD PRESS-KEY KEY SPACE +GOAL> ISA TASK)'''
    )
    return m


actr.clear_state()

# ACT-R tutorial unit7 task for investigating production
# compilation modeling issues.


MODEL = actr.use(build())

val1 = ''
val2 = ''
responses = []
start_time = 0
times = []
exp_length = 0
task_over = True
task_state = None
window = None

result_matrix = [[["win","lose","draw","lose"],["win","lose","draw","lose"],["win","lose","draw","lose"],["lose","draw","win","lose"]],
                 [["win","lose","draw","lose"],["win","lose","draw","lose"],["lose","draw","win","lose"],["lose","draw","win","lose"]],
                 [["win","lose","draw","lose"],["win","lose","draw","lose"],["lose","draw","win","lose"],["lose","draw","win","lose"]],
                 [["win","lose","draw","lose"],["lose","draw","win","lose"],["lose","draw","win","lose"],["lose","draw","win","lose"]]]

def convert_key_to_index(key):
    if key.lower() == 's':
        return 0
    if key.lower() == 'd':
        return 1
    if key.lower() == 'f':
        return 2
    else:
        return 3


def respond_to_key_press (model,key):
    global responses,task_over,times

    if not(task_over):
        if task_state == 'trial':
            ri = convert_key_to_index(key)
            r = result_matrix[val1][val2][ri]
            responses.append(r)
            times.append(actr.get_time() - start_time)
            present_feedback(r)
        elif key.lower() == 'space':
            if len(responses) == exp_length:
                task_over = True
            else:
                present_next_trial()

            
def present_feedback(result):
    global task_state

    task_state = 'feedback'
    actr.clear_exp_window(window)
    actr.add_text_to_exp_window(window, result, x=50, y=100)

def present_next_trial():
    global task_state,start_time,val1,val2

    task_state = 'trial'
    actr.clear_exp_window(window)
    val1 = actr.random(4)
    val2 = actr.random(4)
    actr.add_text_to_exp_window(window, str(val1), x=10, y=50)
    actr.add_text_to_exp_window(window, str(val2), x=110, y=50)
    start_time = actr.get_time()

def game_over():
    return task_over

def trials(n=150,reset=True,output=True):
    global responses,task_over,exp_length,window

    if reset:
        actr.reset()

    window = actr.open_exp_window("Compilation task",visible=False)
    times = []
    responses = []
    task_over = False
    exp_length = n
    present_next_trial()
    actr.install_device(window)

    actr.add_command('compilation-issues-response',respond_to_key_press,"Compilation issues key press response monitor")
    actr.monitor_command('output-key','compilation-issues-response')

    # Just run a long time to have the model perform the task

    actr.run(20000)

    actr.remove_command_monitor('output-key','compilation-issues-response')
    actr.remove_command ('compilation-issues-response')

    return analyze_results(output)

def game(n,show_games=False):
    scores = [0]*15
    times = [0]*15
    for i in range(n):
        r = trials(150,True,show_games)
        scores = list(map(lambda x,y: x + y,scores,r[0]))
        times = list(map(lambda x,y: x + y,times,r[1]))
    print("Average Score of %d trials"%n)
    for x in scores:
        print("%4.2f "%(x / n),end="")
    print()
    print("Average Response times")
    for x in times:
        print("%4.2f "%(x / n / 1000.0),end="")
    print()

def analyze_results(output):
    global responses,times

    r=[]
    t=[]
   
    if output:
        print("Score")

    while len(responses) > 0:
        s = 0
        c = 0
        for i in range(min(10,len(responses))):
            res = responses.pop(0)
            if res == 'win':
                s += 1
            if res == 'lose':
                s -= 1
            c += 1
        if output:
            print("  %2d"%s,end="")
        r.append(s)
    
    if output:
        print()
        print("Average response times")

    while len(times) > 0:
        s = 0
        c = 0
        for i in range(min(10,len(times))):
            s += times.pop(0)
            c += 1
        if output:
            print("%4.2f "%(s / c / 1000.0),end="")
        t.append(s/c)

    if output:
        print()

    return[r,t]


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
