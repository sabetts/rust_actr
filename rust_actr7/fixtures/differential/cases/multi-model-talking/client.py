import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[5]
sys.path.insert(0, str(ROOT / "actr7.x" / "tutorial" / "python"))

import actr


def relay_speech_output(model, string):
    for m in actr.mp_models():
        if m.lower() != model.lower():
            actr.set_current_model(m)
            if not actr.chunk_p(model):
                actr.define_chunks(model)
            actr.new_word_sound(string, actr.mp_time(), model)


actr.load_act_r_model(
    str(ROOT / "actr7.x" / "examples" / "multiple-models" / "multi-model-talking-models.lisp")
)
actr.reset()
actr.add_command("relay-speech-output", relay_speech_output)
actr.monitor_command("output-speech", "relay-speech-output")
for model in actr.mp_models():
    actr.set_current_model(model)
    actr.install_device(["speech", "microphone"])
actr.run(10)
actr.remove_command_monitor("output-speech", "relay-speech-output")
actr.remove_command("relay-speech-output")
