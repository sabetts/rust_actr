import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[5]
sys.path.insert(0, str(ROOT / "actr7.x" / "tutorial" / "python"))

import actr

MODELS = ROOT / "rust_actr7" / "fixtures" / "differential" / "models"
EXTRA_MODEL = MODELS / "probe-multi-model-extra.lisp"
DELETE_CODE = MODELS / "probe-multi-model-delete.lisp"
mutated = False


def mutate(*args):
    global mutated
    if mutated:
        return
    mutated = True
    actr.set_current_model("MP-B")
    actr.set_parameter_value(":RT", -1000)
    actr.set_parameter_value(":ANS", 0.9)
    print("MUTATE MP-B")


def report(label, model):
    actr.set_current_model(model)
    chunk = actr.buffer_read("retrieval")
    value = actr.chunk_slot_value(chunk, "value") if chunk else None
    print(
        label,
        model,
        "TIME",
        actr.get_time(),
        "MP-TIME",
        actr.mp_time(),
        "PARAMS",
        actr.get_parameter_value(":SEED"),
        actr.get_parameter_value(":ANS"),
        actr.get_parameter_value(":RT"),
        actr.get_parameter_value(":V"),
        "RETRIEVAL",
        value,
    )


actr.load_act_r_model(str(MODELS / "probe-multi-model-params.lisp"))
actr.reset()
print("MODELS", actr.mp_models())
for model in actr.mp_models():
    report("BEFORE", model)

actr.add_command("probe-multi-mutate", mutate)
actr.monitor_command("run-start", "probe-multi-mutate")
actr.run(1)
actr.remove_command_monitor("run-start", "probe-multi-mutate")
actr.remove_command("probe-multi-mutate")

for model in actr.mp_models():
    report("AFTER", model)

actr.reload()
print("RELOADED", actr.mp_models())
for model in actr.mp_models():
    report("RELOAD", model)

actr.load_act_r_model(str(EXTRA_MODEL))
print("EXTRA", actr.mp_models())
for model in actr.mp_models():
    report("EXTRA", model)

actr.set_current_model("MP-A")
actr.load_act_r_code(str(DELETE_CODE))
print("DELETED", actr.mp_models())
for model in actr.mp_models():
    report("DELETED", model)

actr.load_act_r_model(str(EXTRA_MODEL))
print("RELOADED-SINGLE", actr.mp_models())
for model in actr.mp_models():
    report("RELOADED-SINGLE", model)
