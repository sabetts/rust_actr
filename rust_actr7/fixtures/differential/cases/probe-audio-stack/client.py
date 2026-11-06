import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[5]
sys.path.insert(0, str(ROOT / "actr7.x" / "tutorial" / "python"))

import actr

MODEL = ROOT / "rust_actr7" / "fixtures" / "differential" / "models" / "probe-audio-stack.lisp"


def show(label):
    print(label)
    actr.print_audicon()


actr.load_act_r_model(str(MODEL))
actr.reset()
actr.set_parameter_value(":v", None)
actr.install_device(["speech", "microphone"])
actr.new_word_sound("gamma", 0.2, "external")
actr.new_digit_sound(7, 0.35)
actr.new_tone_sound(440, 0.1, 0.5)

actr.run(0.1)
show("AUD-100")
actr.run(0.2)
show("AUD-300")
actr.run(0.3)
show("AUD-600")
actr.run(0.4)
show("AUD-1000")
