import actr
import general_import
import sys
import tester

fan = general_import.import_path('../tutorial/python/fan.py',True)


def test_fan_1():
    actr.set_parameter_value(':v',True)
    actr.set_parameter_value(':cmdt',True)
    actr.pbreak('encode-person')


actr.add_command("test-fan-1",test_fan_1,"Testing function for fan model. Do not call.")

def test_fan_2():
    actr.set_parameter_value(':v',True)
    actr.set_parameter_value(':cmdt',True)
    actr.set_parameter_value(':act',True)


actr.add_command("test-fan-2",test_fan_2,"Testing function for fan model. Do not call.")


actr.set_system_parameter_value(':starting-parameters',[':seed',[1,0],':testing-function',"set-testing-traces"])


def run_it():
    sys.stdout = tester.out_file
    fan.sentence("lawyer","store",True,"person")
    actr.sdm('-','arg1','nil','-','arg2','nil')
    actr.set_system_parameter_value(':starting-parameters',[':seed',[1,0],':testing-function',"'test-fan-1'"])
    fan.sentence("lawyer","store",True,"person")
    actr.buffer_chunk('retrieval')
    actr.set_system_parameter_value(':starting-parameters',[':seed',[1,0],':testing-function',"'test-fan-2'"])
    fan.sentence("lawyer","store",True,"person")
    fan.sentence("hippie","bank",True,"person")
    fan.sentence("hippie","bank",True,"location")
    fan.sentence("giant","bank",True,"person")
    fan.sentence("giant","bank",True,"location")
    actr.set_system_parameter_value(':starting-parameters',[':seed',[1,0],':testing-function',"set-testing-traces"])
    fan.experiment()
    sys.stdout = sys.__stdout__    

    actr.remove_command("test-fan-1")
    actr.remove_command("test-fan-2")

