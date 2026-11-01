"""Python port of ACT-R:tutorial;unit3;perceptual-motor-issues-model.lisp plus the pm_issues task."""

from __future__ import annotations

import json
import sys

import rust_actr.actr as actr

DEFAULT_FUNCTION = 'task'
DEFAULT_ARGS = []


def build(seed=(100, 0)) -> actr.Model:
    m = actr.Model('perceptual-motor-issues', seed=seed)

    m.sgp('seed', [101, 0])
    m.sgp('v', True)
    m.sgp('show_focus', True)
    m.sgp('trace_detail', 'medium')
    m.sgp('er', True)

    m.chunk_type('letter', 'name', 'next', 'previous')
    m.chunk_type('task', 'letter')

    m.add_dm(actr.Chunk('a', 'letter', {'name': actr.String('a'), 'next': actr.String('b'), 'previous': actr.String('z')}))
    m.add_dm(actr.Chunk('b', 'letter', {'name': actr.String('b'), 'next': actr.String('c'), 'previous': actr.String('a')}))
    m.add_dm(actr.Chunk('c', 'letter', {'name': actr.String('c'), 'next': actr.String('d'), 'previous': actr.String('b')}))
    m.add_dm(actr.Chunk('d', 'letter', {'name': actr.String('d'), 'next': actr.String('e'), 'previous': actr.String('c')}))
    m.add_dm(actr.Chunk('e', 'letter', {'name': actr.String('e'), 'next': actr.String('f'), 'previous': actr.String('d')}))
    m.add_dm(actr.Chunk('f', 'letter', {'name': actr.String('f'), 'next': actr.String('g'), 'previous': actr.String('e')}))
    m.add_dm(actr.Chunk('g', 'letter', {'name': actr.String('g'), 'next': actr.String('h'), 'previous': actr.String('f')}))
    m.add_dm(actr.Chunk('h', 'letter', {'name': actr.String('h'), 'next': actr.String('i'), 'previous': actr.String('g')}))
    m.add_dm(actr.Chunk('i', 'letter', {'name': actr.String('i'), 'next': actr.String('j'), 'previous': actr.String('h')}))
    m.add_dm(actr.Chunk('j', 'letter', {'name': actr.String('j'), 'next': actr.String('k'), 'previous': actr.String('i')}))
    m.add_dm(actr.Chunk('k', 'letter', {'name': actr.String('k'), 'next': actr.String('l'), 'previous': actr.String('j')}))
    m.add_dm(actr.Chunk('l', 'letter', {'name': actr.String('l'), 'next': actr.String('m'), 'previous': actr.String('k')}))
    m.add_dm(actr.Chunk('m', 'letter', {'name': actr.String('m'), 'next': actr.String('n'), 'previous': actr.String('l')}))
    m.add_dm(actr.Chunk('n', 'letter', {'name': actr.String('n'), 'next': actr.String('o'), 'previous': actr.String('m')}))
    m.add_dm(actr.Chunk('o', 'letter', {'name': actr.String('o'), 'next': actr.String('p'), 'previous': actr.String('n')}))
    m.add_dm(actr.Chunk('p', 'letter', {'name': actr.String('p'), 'next': actr.String('q'), 'previous': actr.String('o')}))
    m.add_dm(actr.Chunk('q', 'letter', {'name': actr.String('q'), 'next': actr.String('r'), 'previous': actr.String('p')}))
    m.add_dm(actr.Chunk('r', 'letter', {'name': actr.String('r'), 'next': actr.String('s'), 'previous': actr.String('q')}))
    m.add_dm(actr.Chunk('s', 'letter', {'name': actr.String('s'), 'next': actr.String('t'), 'previous': actr.String('r')}))
    m.add_dm(actr.Chunk('t', 'letter', {'name': actr.String('t'), 'next': actr.String('u'), 'previous': actr.String('s')}))
    m.add_dm(actr.Chunk('u', 'letter', {'name': actr.String('u'), 'next': actr.String('v'), 'previous': actr.String('t')}))
    m.add_dm(actr.Chunk('v', 'letter', {'name': actr.String('v'), 'next': actr.String('w'), 'previous': actr.String('u')}))
    m.add_dm(actr.Chunk('w', 'letter', {'name': actr.String('w'), 'next': actr.String('x'), 'previous': actr.String('v')}))
    m.add_dm(actr.Chunk('x', 'letter', {'name': actr.String('x'), 'next': actr.String('y'), 'previous': actr.String('w')}))
    m.add_dm(actr.Chunk('y', 'letter', {'name': actr.String('y'), 'next': actr.String('z'), 'previous': actr.String('x')}))
    m.add_dm(actr.Chunk('z', 'letter', {'name': actr.String('z'), 'next': actr.String('a'), 'previous': actr.String('y')}))
    m.add_chunk('start', 'chunk')
    m.add_chunk('attend', 'chunk')
    m.add_chunk('respond', 'chunk')
    m.add_chunk('done', 'chunk')

    m.add_production(
        '''
        (p find-letter
           =visual-location>
           ?visual>
             state       free
         ==>
           +visual>
             cmd         move-attention
             screen-pos  =visual-location
           +imaginal>
        )
        '''
    )

    m.add_production(
        '''
        (p encode-letter
           =imaginal>
             isa         task
             letter      nil
           =visual>
             isa         visual-object
             value       =letter
         ==>
           =imaginal>
             letter      =letter
        )
        '''
    )

    m.add_production(
        '''
        (p respond-next
           =imaginal>
             isa         task
             letter      =letter
           =visual>
             isa         visual-object
             value       "next"
           ?manual>
             state       busy
         ==>
           +retrieval>
             isa         letter
             previous    =letter
           +manual>
             cmd         press-key
             key         =letter
        )
        '''
    )

    m.add_production(
        '''
        (p respond-previous
           =imaginal>
             isa         task
             letter      =letter
           =visual>
             isa         text
             value       "previous"
           ?manual>
             state       free
         ==>
           +retrieval>
             isa         letter
             next        =letter
           +manual>
             cmd         press-key
             key         =letter
        )
        '''
    )

    m.add_production(
        '''
        (p respond-final
           =retrieval>
             isa         letter
             name        =letter
           ==>
           +manual>
             cmd         press-key
             key         =letter
        )
        '''
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
