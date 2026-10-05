import actr
import io
from contextlib import redirect_stdout

backup = actr.current_connection

def run_it():

    actr.current_connection = None
    f=io.StringIO()
    with redirect_stdout(f):
        actr.current_connection = actr.connection()
    actr.stop()
    actr.current_connection = backup
    actr.model_output(">>> import actr")
    actr.model_output(f.getvalue()[0:-1])
    f.close()
    # If I actually load here I get the ACL load messages...
    actr.model_output('>>> actr.load_act_r_model("ACT-R:tutorial;unit1;addition.lisp")')
    actr.model_output(str(load_result))
    actr.model_output(">>> actr.run(1)")
    vals = actr.run(1)
    actr.model_output(str(vals))
    actr.model_output(">>> actr.reset()")
    vals = actr.reset()
    actr.model_output(str(vals))
    actr.model_output(">>> actr.run(.1)")
    vals = actr.run(.1)
    actr.model_output(str(vals))
    # Cheat and don't actually reload just output the
    # same true value from the initial loading since
    # again ACL is going to print the load details...
    actr.model_output(">>> actr.reload()")
    actr.model_output(str(load_result))


load_result = actr.load_act_r_model("ACT-R:tutorial;unit1;addition.lisp")
