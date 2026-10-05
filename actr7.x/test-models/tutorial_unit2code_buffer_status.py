import actr


def run_it():

    actr.model_output(">>> actr.buffer_status('goal','visual')")
    vals = actr.buffer_status('goal','visual')
    actr.model_output(str(vals))

load_result = actr.load_act_r_model("ACT-R:tutorial;unit1;addition.lisp")
