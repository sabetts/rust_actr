import actr
import io
from contextlib import redirect_stdout

backup = actr.current_connection

def run_it():

    actr.model_output(">>> actr.sdm('-','addend1','1')")
    vals = actr.sdm('-','addend1','1')
    actr.model_output(str(vals))


load_result = actr.load_act_r_model("ACT-R:tutorial;unit1;tutor-model-solution.lisp")
