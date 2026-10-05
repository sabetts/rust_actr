import actr
import general_import
import sys
import tester

fan_no_pm = general_import.import_path('../tutorial/python/fan_no_pm.py',True)


def test_fan_2():
    actr.set_parameter_value(':v',True)
    actr.set_parameter_value(':cmdt',True)
    actr.set_parameter_value(':act',True)


actr.add_command("test-fan-2",test_fan_2,"Testing function for fan model. Do not call.")


actr.set_system_parameter_value(':starting-parameters',[':seed',[1,0],':testing-function',"set-testing-traces"])


def run_it():
    sys.stdout = tester.out_file
    fan_no_pm.sentence("'lawyer'","'store'",True,"person")
    actr.sdm('-','arg1','nil','-','arg2','nil')
    actr.set_system_parameter_value(':starting-parameters',[':seed',[1,0],':testing-function',"'test-fan-2'"])
    fan_no_pm.sentence("'lawyer'","'store'",True,"person")
    fan_no_pm.sentence("'hippie'","'bank'",True,"person")
    fan_no_pm.sentence("'hippie'","'bank'",True,"location")
    fan_no_pm.sentence("'giant'","'bank'",True,"person")
    fan_no_pm.sentence("'giant'","'bank'",True,"location")
    actr.set_system_parameter_value(':starting-parameters',[':seed',[1,0],':testing-function',"set-testing-traces"])
    fan_no_pm.experiment()
    sys.stdout = sys.__stdout__    

    actr.remove_command("test-fan-2")

