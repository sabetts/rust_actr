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
    / "probe-retrieval-lifecycle.lisp"
)

actr.load_act_r_model(str(MODEL))
actr.reset()
actr.run(1)
