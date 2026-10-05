import actr
import io
from contextlib import redirect_stdout

backup = actr.current_connection

def run_it():

    actr.model_output(">>> actr.dm()")
    vals = actr.dm()
    actr.model_output(str(vals))
    actr.model_output(">>> actr.dm('one','three')")
    vals = actr.dm('one','three')
    actr.model_output(str(vals))


load_result = actr.load_act_r_model("ACT-R:tutorial;unit1;addition.lisp")
