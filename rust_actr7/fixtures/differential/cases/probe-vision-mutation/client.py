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
    / "probe-vision-mutation.lisp"
)


def show(label):
    print(label)
    print("TIME", actr.get_time())
    actr.print_visicon()


actr.load_act_r_model(str(MODEL))
actr.reset()

window = actr.open_exp_window(
    "probe vision mutation", visible=False, width=200, height=120, x=0, y=0
)
actr.install_device(window)
alpha = actr.add_text_to_exp_window(window, "alpha", x=20, y=30, color="red")
actr.add_text_to_exp_window(window, "beta", x=60, y=30, color="black")
actr.run(0.05)
show("INITIAL")

actr.mod_focus("state", 1)
actr.run(0.2)
show("NEAREST-BEFORE")

actr.remove_items_from_exp_window(window, alpha)
actr.run(0.05)
actr.mod_focus("state", 3)
actr.run(0.2)
show("NEAREST-AFTER-REMOVE")

actr.add_text_to_exp_window(window, "alpha", x=20, y=70, color="blue")
actr.run(0.05)
actr.mod_focus("state", 5)
actr.run(0.2)
show("TWO-FEATURE")

actr.clear_exp_window(window)
actr.run(0.05)
show("AFTER-CLEAR-EMPTY")
actr.run_until_time(0.35)

actr.add_text_to_exp_window(window, "gamma", x=90, y=70, color="green")
actr.run(0.05)
actr.mod_focus("state", 7)
actr.run(0.2)
show("NEAREST-AFTER-CLEAR")
