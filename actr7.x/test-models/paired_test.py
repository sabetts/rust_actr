import actr
import general_import

paired = general_import.import_path('../tutorial/python/paired.py',True)

act_trace = False
compilation = False

def test_paired():
    actr.set_parameter_value(':v',True)
    actr.set_parameter_value(':cmdt',True)
    actr.set_parameter_value(':act',act_trace)
    actr.set_parameter_value(':epl',compilation)
    actr.set_parameter_value(':pct',compilation)

actr.add_command("test-paired",test_paired,"Testing function for paired model. Do not call.")

actr.set_system_parameter_value(':starting-parameters',[':testing-function',"'test-paired'"])


def run_it():
    actr.record_history("buffer-trace")
    actr.record_history("goal","visual","visual-location","imaginal","retrieval","manual")

    global act_trace
    global compilation
    
    act_trace = True
    compilation = False
    paired.task(1,2)
    actr.sdm('isa','pair')
    act_trace = False
    compilation = True
    paired.task(1,2)
    act_trace = False
    compilation = False
    paired.task(2,2)
    actr.predict_bold_response()
    actr.remove_command("test-paired")
