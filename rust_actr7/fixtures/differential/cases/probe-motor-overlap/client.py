import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[5]
sys.path.insert(0, str(ROOT / "actr7.x" / "tutorial" / "python"))

import actr

MODEL = (
    ROOT
    / "rust_actr7"
    / "fixtures"
    / "differential"
    / "models"
    / "probe-motor-overlap.lisp"
)

actr.load_act_r_model(str(MODEL))
actr.reset()
actr.install_device(["motor", "keyboard"])

actr.run(0.1)
print("TIME", actr.get_time())
actr.mod_focus("state", 1)
actr.run(0.2)
print("TIME", actr.get_time())
actr.mod_focus("state", 2)
actr.run(0.3)
print("TIME", actr.get_time())
