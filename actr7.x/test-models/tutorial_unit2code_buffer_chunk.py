import actr
import io
from contextlib import redirect_stdout

backup = actr.current_connection

def run_it():

    actr.set_parameter_value(":v",None)
    actr.run(1)
    actr.set_parameter_value(":v",True)
    actr.model_output(">>> actr.buffer_chunk('goal','visual')")
    vals = actr.buffer_chunk('goal','visual')
    actr.model_output(str(vals))

load_result = actr.load_act_r_model("ACT-R:tutorial;unit1;addition.lisp")
