import actr
import sys
import general_import
import tester

siegler = general_import.import_path('../tutorial/python/siegler.py',True)

actr.set_system_parameter_value(':starting-parameters',[':seed',[1,0],':testing-function',"set-testing-traces"])


def run_it():
    sys.stdout = tester.out_file
    siegler.experiment(2)
    sys.stdout = sys.__stdout__    


