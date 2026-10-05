import actr
import general_import

sperling = general_import.import_path('../tutorial/python/sperling.py',True)

trace_level = 'low'

def output_visicon ():
    actr.print_visicon()
    actr.remove_command_monitor("visicon-update","sperling-test-visicon-printer")

actr.add_command("sperling-test-visicon-printer",output_visicon,"Testing function for sperling model. Do not call.")

def test_sperling():
    actr.set_parameter_value(':v',True)
    actr.set_parameter_value(':cmdt',True)
    actr.set_parameter_value(':trace-detail',trace_level)
    actr.monitor_command("visicon-update","sperling-test-visicon-printer")

actr.add_command("test-sperling",test_sperling,"Testing function for sperling model. Do not call.")


actr.set_system_parameter_value(':starting-parameters',[':testing-function',"'test-sperling'"])


def run_it():
    global trace_level
    trace_level = 'low'
    sperling.trial(.15)
    trace_level = 'high'
    sperling.trial(.15)
    actr.remove_command("test-sperling")
    actr.remove_command("sperling-test-visicon-printer")
