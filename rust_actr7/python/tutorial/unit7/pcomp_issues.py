"""Python port of ACT-R:tutorial;unit7;production-compilation-issues-model.lisp plus the pcomp_issues task."""

from __future__ import annotations

import json
import sys

import rust_actr.actr as actr

DEFAULT_FUNCTION = 'game'
DEFAULT_ARGS = [1]


def build(seed=(100, 0)) -> actr.Model:
    m = actr.Model('compilation-test', seed=seed)

    m.sgp('esc', True)
    m.sgp('lf', 0.5)
    m.sgp('bll', 0.5)
    m.sgp('mp', 18)
    m.sgp('rt', -3)
    m.sgp('ans', 0.25)
    m.sgp('style_warnings', None)
    m.sgp('v', None)

    m.chunk_type('task', 'state')
    m.chunk_type('number', 'visual-rep')
    m.chunk_type('response', 'key', 'is-response', defaults={'is_response': True})
    m.chunk_type('trial', 'num1', 'num2', 'result', 'response')

    m.define_chunks('win')
    m.define_chunks('draw')
    m.define_chunks('lose')
    m.define_chunks('attend-num-1')
    m.define_chunks('encode-num-1')
    m.define_chunks('find-num-2')
    m.define_chunks('attend-num-2')
    m.define_chunks('encode-num-2')
    m.define_chunks('num2')
    m.define_chunks('retrieve-past-trial')
    m.define_chunks('process-past-trial')
    m.define_chunks('respond')
    m.define_chunks('guess-other')
    m.define_chunks('detect-feedback')
    m.define_chunks('encode-feedback')
    m.add_chunk('zero', 'number', visual_rep=actr.String('0'))
    m.add_chunk('one', 'number', visual_rep=actr.String('1'))
    m.add_chunk('two', 'number', visual_rep=actr.String('2'))
    m.add_chunk('three', 'number', visual_rep=actr.String('3'))
    m.add_chunk('response-1', 'response', key=actr.String('s'))
    m.add_chunk('response-2', 'response', key=actr.String('d'))
    m.add_chunk('response-3', 'response', key=actr.String('f'))

    m.set_similarities([('win', 'draw', -0.2), ('win', 'lose', -0.15), ('zero', 'one', -0.1), ('one', 'two', -0.1), ('two', 'three', -0.1), ('zero', 'two', -0.2), ('one', 'three', -0.2), ('zero', 'three', -0.3)])
    m.set_all_base_levels(1000, -100)

    m.goal_focus(isa='task')

    m.add_production(
        '''
        (p detect-trial-start
           =goal>
             isa task
             state nil
           =visual-location>
             isa visual-location
           ?visual>
             state free
           ?imaginal>
             state free
           ==>
           =goal>
             state attend-num-1
           +imaginal>
             isa trial
           +visual>
             cmd move-attention
             screen-pos =visual-location)
        '''
    )

    m.add_production(
        '''
        (p attend-num-1
           =goal>
             isa task
             state attend-num-1
           =visual>
             isa visual-object
             value =val
           ==>
           =goal>
             state encode-num-1
           +retrieval>
             isa number
             visual-rep =val)
        '''
    )

    m.add_production(
        '''
        (p encode-num-1
           =goal>
             isa task
             state encode-num-1
           =retrieval>
             isa number
           =imaginal>
             isa trial
           ==>
           =imaginal>
             num1 =retrieval
           =goal>
             state find-num-2
           +visual-location>
             isa visual-location
             > screen-x current
             :attended nil)
        '''
    )

    m.add_production(
        '''
        (p find-num-2
           =goal>
             isa task
             state find-num-2
           =visual-location>
             isa visual-location
           ?visual>
             state free
           ==>
           =goal>
             state attend-num-2
           +visual>
             cmd move-attention
             screen-pos =visual-location)
        '''
    )

    m.add_production(
        '''
        (p attend-num-2
           =goal>
             isa task
             state attend-num-2
           =visual>
             isa visual-object
             value =val
           ==>
           =goal>
             state encode-num-2
           +retrieval>
             isa number
             visual-rep =val)
        '''
    )

    m.add_production(
        '''
        (p encode-num-2
           =goal>
             isa task
             state encode-num-2
           =retrieval>
             isa number
           =imaginal>
             isa trial
           ==>
           =imaginal>
             num2 =retrieval
           =goal>
             state retrieve-past-trial)
        '''
    )

    m.add_production(
        '''
        (p retrieve-past-trial
           =goal>
             isa task
             state retrieve-past-trial
           =imaginal>
             isa trial
             num1 =n1
             num2 =n2
           ==>
           =imaginal>
           +retrieval>
             isa trial
             num1 =n1
             num2 =n2
             result win
           =goal>
             state process-past-trial)
        '''
    )

    m.add_production(
        '''
        (p no-past-trial
           =goal>
             isa task
             state process-past-trial
           ?retrieval>
             buffer failure
           ==>
           +retrieval>
             isa response
           =goal>
             state respond)
        '''
    )

    m.add_production(
        '''
        (p retrieved-a-win
           =goal>
             isa task
             state process-past-trial
           =retrieval>
             isa trial
             result win
             response =response
           ==>
           +retrieval> =response
           =goal>
             state respond)
        '''
    )

    m.add_production(
        '''
        (p retrieved-a-non-win
           =goal>
             isa task
             state process-past-trial
           =retrieval>
             isa trial
            - result win
             response =response
           ==>
           +retrieval> =response
           =goal>
             state guess-other)
        '''
    )

    m.add_production(
        '''
        (p guess-other
           =goal>
             isa task
             state guess-other
           =retrieval>
             isa response
             key =key
           ==>
           +retrieval>
             isa response
            - key =key
           =goal>
             state respond)
        '''
    )

    m.add_production(
        '''
        (p respond
           =goal>
             isa task
             state respond
           =retrieval>
             isa response
             key =key
           ?manual>
             state free
           =imaginal>
             isa trial
           ==>
           =imaginal>
             response =retrieval
           +manual>
             cmd press-key
             key =key
           =goal>
             state detect-feedback)
        '''
    )

    m.add_production(
        '''
        (p respond-when-response-failure
           =goal>
             isa task
             state respond
           ?retrieval>
             buffer failure
           ?manual>
             state free
           =imaginal>
             isa trial
           ==>
           =imaginal>
             response response-2
           +manual>
             cmd press-key
             key "d"
           =goal>
             state detect-feedback)
        '''
    )

    m.add_production(
        '''
        (p detect-feedback
           =goal>
             isa task
             state detect-feedback
           =visual-location>
             isa visual-location
           ?visual>
             state free
           ==>
           +visual>
             cmd move-attention
             screen-pos =visual-location
           =goal>
             state encode-feedback)
        '''
    )

    m.add_production(
        '''
        (p encode-feedback-win
           =goal>
             isa task
             state encode-feedback
           =imaginal>
             isa trial
           =visual>
             isa visual-object
             value "win"
           ?manual>
             state free
           ==>
           =imaginal>
             result win
           +manual>
             cmd press-key
             key space
           +goal>
             isa task)
        '''
    )

    m.add_production(
        '''
        (p encode-feedback-lose
           =goal>
             isa task
             state encode-feedback
           =imaginal>
             isa trial
           =visual>
             isa visual-object
             value "lose"
           ?manual>
             state free
           ==>
           =imaginal>
             result lose
           +manual>
             cmd press-key
             key space
           +goal>
             isa task)
        '''
    )

    m.add_production(
        '''
        (p encode-feedback-draw
         =goal>
           isa task
           state encode-feedback
         =imaginal>
           isa trial
         =visual>
           isa visual-object
           value "draw"
         ?manual>
           state free
         ==>
         =imaginal>
           result draw
         +manual>
           cmd press-key
           key space
         +goal>
           isa task)
        '''
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
