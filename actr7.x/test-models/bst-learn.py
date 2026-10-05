import actr
import general_import
import sys
import tester

bst = general_import.import_path('../tutorial/python/bst.py',True)


actr.set_system_parameter_value(':starting-parameters',[':seed',[1,0],':model-warnings',False,':testing-function',"set-testing-traces"])


def run_it():
    sys.stdout = tester.out_file
    bst.experiment(1)
    sys.stdout = sys.__stdout__    
