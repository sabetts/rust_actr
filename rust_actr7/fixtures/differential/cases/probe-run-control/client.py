import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[5]
sys.path.insert(0, str(ROOT / "actr7.x" / "tutorial" / "python"))

import actr

MODEL = ROOT / "rust_actr7" / "fixtures" / "differential" / "models" / "probe-run-control.lisp"


def report(label, result):
    print(label, result, actr.get_time())


actr.load_act_r_model(str(MODEL))
actr.reset()

report("N2", actr.run_n_events(2))
report("U300", actr.run_until_time(0.3))

actr.reset()
actr.schedule_break_relative(0.12)
report("B1", actr.run(1))
report("B2", actr.run(1))

actr.reset()
report("F50", actr.run_full_time(0.05))
report("U500", actr.run_until_time(0.5))
