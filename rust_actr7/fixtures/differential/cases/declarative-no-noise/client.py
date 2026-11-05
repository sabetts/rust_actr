import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[5]
sys.path.insert(0, str(ROOT / "actr7.x" / "tutorial" / "python"))

import actr

actr.load_act_r_model(str(ROOT / "actr7.x" / "tutorial" / "unit5" / "declarative-issues.lisp"))
actr.set_parameter_value(":ans", None)
actr.run(1.0)
