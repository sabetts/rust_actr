import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[5]
sys.path.insert(0, str(ROOT / "actr7.x" / "tutorial" / "python"))

import actr

MODEL = ROOT / "rust_actr7" / "fixtures" / "differential" / "models" / "probe-imaginal-indirect.lisp"


def slot(buffer, name):
    chunk = actr.buffer_read(buffer)
    if chunk is None:
        return None
    return actr.chunk_slot_value(chunk, name)


def show(label):
    print(label)
    print("IMAGINAL", slot("imaginal", "slot1"), slot("imaginal", "slot2"))
    print("RETRIEVAL", slot("retrieval", "slot1"), slot("retrieval", "slot2"))
    print("GOAL", slot("goal", "state"))
    print("TIME", actr.get_time())


actr.load_act_r_model(str(MODEL))
actr.reset()
actr.run(0.1)
show("IMG-100")
actr.run(0.4)
show("IMG-500")
actr.run(0.4)
show("IMG-900")
actr.run(0.4)
show("IMG-1300")
actr.run(0.4)
show("IMG-1700")
