import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[5]
sys.path.insert(0, str(ROOT / "actr7.x" / "tutorial" / "python"))

import actr

MODEL = ROOT / "rust_actr7" / "fixtures" / "differential" / "models" / "probe-audio-trace.lisp"

actr.load_act_r_model(str(MODEL))
actr.reset()
actr.install_device(["speech", "microphone"])
actr.new_word_sound("gamma", 0.2, "external")
actr.new_digit_sound(7, 0.35)
actr.new_tone_sound(440, 0.1, 0.5)

actr.run(0.1)
actr.run(0.2)
actr.run(0.3)
actr.run(0.6)
