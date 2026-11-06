import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[5]
sys.path.insert(0, str(ROOT / "actr7.x" / "tutorial" / "python"))

import actr

MODEL = ROOT / "rust_actr7" / "fixtures" / "differential" / "models" / "probe-compile-utility.lisp"


def report(label):
    print(label)
    print("COMPILED")
    actr.pp("PRODUCTION0")
    print("U-A", actr.no_output("spp", "dynamic-a", ":u"))
    print("U-B", actr.no_output("spp", "dynamic-b", ":u"))
    print("U-COMPILED", actr.no_output("spp", "PRODUCTION0", ":u"))
    print("TIME", actr.get_time())


actr.load_act_r_model(str(MODEL))
actr.reset()
actr.run(1)
report("FIRST")

actr.pdisable("dynamic-a")
actr.mod_focus("step", "one", "val", "color")
actr.run(1)
report("SECOND")
