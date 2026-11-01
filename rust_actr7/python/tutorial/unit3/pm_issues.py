"""Python port of ACT-R:tutorial;unit3;perceptual-motor-issues-model.lisp plus the pm_issues task."""

from __future__ import annotations

import json
import sys

import rust_actr.actr as actr

LOGICAL_PATH = 'ACT-R:tutorial;unit3;perceptual-motor-issues-model.lisp'
MODEL_NAME = 'PERCEPTUAL-MOTOR-ISSUES'
DEFAULT_FUNCTION = 'task'
DEFAULT_ARGS = []


def build(seed=(100, 0)) -> actr.Model:
    m = actr.Model('PERCEPTUAL-MOTOR-ISSUES', seed=seed)
    m.sgp(':SEED', [101, 0])
    m.sgp(':V', True)
    m.sgp(':SHOW-FOCUS', True)
    m.sgp(':TRACE-DETAIL', 'MEDIUM')
    m.sgp(':ER', True)
    m.chunk_type('LETTER', 'NAME', 'NEXT', 'PREVIOUS')
    m.chunk_type('TASK', 'LETTER')
    m.add_chunk('A', 'LETTER', **{'NAME': actr.String('a'), 'NEXT': actr.String('b'), 'PREVIOUS': actr.String('z')})
    m.add_chunk('B', 'LETTER', **{'NAME': actr.String('b'), 'NEXT': actr.String('c'), 'PREVIOUS': actr.String('a')})
    m.add_chunk('C', 'LETTER', **{'NAME': actr.String('c'), 'NEXT': actr.String('d'), 'PREVIOUS': actr.String('b')})
    m.add_chunk('D', 'LETTER', **{'NAME': actr.String('d'), 'NEXT': actr.String('e'), 'PREVIOUS': actr.String('c')})
    m.add_chunk('E', 'LETTER', **{'NAME': actr.String('e'), 'NEXT': actr.String('f'), 'PREVIOUS': actr.String('d')})
    m.add_chunk('F', 'LETTER', **{'NAME': actr.String('f'), 'NEXT': actr.String('g'), 'PREVIOUS': actr.String('e')})
    m.add_chunk('G', 'LETTER', **{'NAME': actr.String('g'), 'NEXT': actr.String('h'), 'PREVIOUS': actr.String('f')})
    m.add_chunk('H', 'LETTER', **{'NAME': actr.String('h'), 'NEXT': actr.String('i'), 'PREVIOUS': actr.String('g')})
    m.add_chunk('I', 'LETTER', **{'NAME': actr.String('i'), 'NEXT': actr.String('j'), 'PREVIOUS': actr.String('h')})
    m.add_chunk('J', 'LETTER', **{'NAME': actr.String('j'), 'NEXT': actr.String('k'), 'PREVIOUS': actr.String('i')})
    m.add_chunk('K', 'LETTER', **{'NAME': actr.String('k'), 'NEXT': actr.String('l'), 'PREVIOUS': actr.String('j')})
    m.add_chunk('L', 'LETTER', **{'NAME': actr.String('l'), 'NEXT': actr.String('m'), 'PREVIOUS': actr.String('k')})
    m.add_chunk('M', 'LETTER', **{'NAME': actr.String('m'), 'NEXT': actr.String('n'), 'PREVIOUS': actr.String('l')})
    m.add_chunk('N', 'LETTER', **{'NAME': actr.String('n'), 'NEXT': actr.String('o'), 'PREVIOUS': actr.String('m')})
    m.add_chunk('O', 'LETTER', **{'NAME': actr.String('o'), 'NEXT': actr.String('p'), 'PREVIOUS': actr.String('n')})
    m.add_chunk('P', 'LETTER', **{'NAME': actr.String('p'), 'NEXT': actr.String('q'), 'PREVIOUS': actr.String('o')})
    m.add_chunk('Q', 'LETTER', **{'NAME': actr.String('q'), 'NEXT': actr.String('r'), 'PREVIOUS': actr.String('p')})
    m.add_chunk('R', 'LETTER', **{'NAME': actr.String('r'), 'NEXT': actr.String('s'), 'PREVIOUS': actr.String('q')})
    m.add_chunk('S', 'LETTER', **{'NAME': actr.String('s'), 'NEXT': actr.String('t'), 'PREVIOUS': actr.String('r')})
    m.add_chunk('T', 'LETTER', **{'NAME': actr.String('t'), 'NEXT': actr.String('u'), 'PREVIOUS': actr.String('s')})
    m.add_chunk('U', 'LETTER', **{'NAME': actr.String('u'), 'NEXT': actr.String('v'), 'PREVIOUS': actr.String('t')})
    m.add_chunk('V', 'LETTER', **{'NAME': actr.String('v'), 'NEXT': actr.String('w'), 'PREVIOUS': actr.String('u')})
    m.add_chunk('W', 'LETTER', **{'NAME': actr.String('w'), 'NEXT': actr.String('x'), 'PREVIOUS': actr.String('v')})
    m.add_chunk('X', 'LETTER', **{'NAME': actr.String('x'), 'NEXT': actr.String('y'), 'PREVIOUS': actr.String('w')})
    m.add_chunk('Y', 'LETTER', **{'NAME': actr.String('y'), 'NEXT': actr.String('z'), 'PREVIOUS': actr.String('x')})
    m.add_chunk('Z', 'LETTER', **{'NAME': actr.String('z'), 'NEXT': actr.String('a'), 'PREVIOUS': actr.String('y')})
    m.add_chunk('START', 'CHUNK', **{})
    m.add_chunk('ATTEND', 'CHUNK', **{})
    m.add_chunk('RESPOND', 'CHUNK', **{})
    m.add_chunk('DONE', 'CHUNK', **{})
    m.add_production(
        '''(P FIND-LETTER =VISUAL-LOCATION> ?VISUAL> STATE FREE ==> +VISUAL> CMD MOVE-ATTENTION SCREEN-POS =VISUAL-LOCATION +IMAGINAL>)'''
    )
    m.add_production(
        '''(P ENCODE-LETTER =IMAGINAL> ISA TASK LETTER NIL =VISUAL> ISA VISUAL-OBJECT VALUE =LETTER ==> =IMAGINAL> LETTER =LETTER)'''
    )
    m.add_production(
        '''(P RESPOND-NEXT =IMAGINAL> ISA TASK LETTER =LETTER =VISUAL> ISA VISUAL-OBJECT VALUE "next" ?MANUAL> STATE BUSY ==> +RETRIEVAL> ISA LETTER PREVIOUS =LETTER +MANUAL> CMD PRESS-KEY KEY =LETTER)'''
    )
    m.add_production(
        '''(P RESPOND-PREVIOUS =IMAGINAL> ISA TASK LETTER =LETTER =VISUAL> ISA TEXT VALUE "previous" ?MANUAL> STATE FREE ==> +RETRIEVAL> ISA LETTER NEXT =LETTER +MANUAL> CMD PRESS-KEY KEY =LETTER)'''
    )
    m.add_production(
        '''(P RESPOND-FINAL =RETRIEVAL> ISA LETTER NAME =LETTER ==> +MANUAL> CMD PRESS-KEY KEY =LETTER)'''
    )
    return m


actr.clear_state()

# ACT-R tutorial unit 3 code for a simple task to 
# investigate potential perceptual and motor issues
# with models.


MODEL = actr.use(build())

response = []
response_time = False


def respond_to_key_press (model,key):
    global response,response_time

    if not(response_time):
        response_time = actr.get_time()
    response.append(key.upper())


def task(which=False):
    global response,response_time
   
    actr.reset()
    alphabet = ["A","B","C","D","E","F","G","H",
                "I","J","K","L","M","N","O","P","Q",
                "R","S","T","U","V","W","X","Y","Z"]
    letter = actr.permute_list(alphabet)[0]
    alphabet = ["Z"] + alphabet

    if which == 'next' or which == 'previous':
        task = which
    elif actr.random(2) == 0:
        task = 'next'
    else: 
        task = 'previous'

    time = 1500 + actr.random(1000)

    window = actr.open_exp_window("Simple task")

    actr.install_device(window)
   
    actr.add_command("pm-issues-response",respond_to_key_press,
                     "Perceptual-motor issues task response")
    actr.monitor_command("output-key","pm-issues-response")

    actr.add_text_to_exp_window(window, letter, x=130, y=150)

    actr.add_command("pm-issue-display",display_prompt,
                     "Perceptual-motor issues task prompt display")

    actr.schedule_event_relative(time,"pm-issue-display",params=[window,task],time_in_ms=True)

    response = []
    response_time = False


    actr.run(10,True)    

    actr.remove_command("pm-issue-display")
    actr.remove_command_monitor("output-key","pm-issues-response")
    actr.remove_command("pm-issues-response")

    if (len(response) == 2 and response_time > time and response[0] == letter and
         ((task == 'next' and alphabet.index(response[0]) == (alphabet.index(response[1]) - 1)) or
          (task == 'previous' and alphabet.index(response[0]) == (alphabet.index(response[1]) + 1)))):
        result = True
    else:
        result = False

    return [task,result]

def display_prompt(window,prompt):

    actr.clear_exp_window()
    actr.add_text_to_exp_window(window, prompt, x=125, y=150)


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
