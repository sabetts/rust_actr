"""Python port of ACT-R:tutorial;unit6;choice-model.lisp plus the choice task."""

from __future__ import annotations

import json
import sys

import rust_actr.actr as actr

DEFAULT_FUNCTION = None
DEFAULT_ARGS = []


def build(seed=(100, 0)) -> actr.Model:
    m = actr.Model('choice', seed=seed)

    return m


actr.clear_state()


MODEL = actr.use(build())

choice_data = [0.664, 0.778, 0.804, 0.818]

response = False

def respond_to_key_press (model,key):
    global response

    response = key

def person():
    global response

    if actr.visible_virtuals_available():
        window = actr.open_exp_window("Choice Experiment",visible=True)
    
        actr.add_command("choice-response",respond_to_key_press,"Choice task key response")
        actr.monitor_command("output-key","choice-response")

        actr.add_text_to_exp_window (window, 'choose', x=50, y=100)

        response = ''
            
        while response == '':
            actr.process_events()

        actr.clear_exp_window(window)
    
        if actr.random(1.0) < .9:
            answer = 'heads'
        else:
            answer = 'tails'

        actr.add_text_to_exp_window (window, answer, x=50, y=100)

        start = actr.get_time(False)
            
        while (actr.get_time(False) - start) < 1000:
            actr.process_events()

        actr.remove_command_monitor("output-key","choice-response")
        actr.remove_command("choice-response")
 
        return response


def model ():
    None

def data (n):
    None


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
