import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[5]
sys.path.insert(0, str(ROOT / "actr7.x" / "tutorial" / "python"))

import actr

MODEL = ROOT / "rust_actr7" / "fixtures" / "differential" / "models" / "probe-runtime-defs.lisp"

mutated = False


def mutate(*args):
    global mutated
    text = str(args[-1]).strip() if args else ""
    if mutated or text != '"FIRST"':
        return
    mutated = True
    actr.add_dm(["runtime", "isa", "probe-goal", "state", 2])
    actr.define_chunks(["defined", "isa", "probe-goal", "state", 3])


actr.load_act_r_model(str(MODEL))
actr.reset()
actr.add_command("probe-runtime-mutate", mutate)
actr.monitor_command("model-output", "probe-runtime-mutate")
actr.run(1)
actr.remove_command_monitor("model-output", "probe-runtime-mutate")
actr.remove_command("probe-runtime-mutate")
