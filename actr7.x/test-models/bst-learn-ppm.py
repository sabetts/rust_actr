import actr
import general_import
import sys
import tester

bst_ppm = general_import.import_path('../tutorial/python/bst_ppm.py',True)


actr.set_system_parameter_value(':starting-parameters',[':seed',[1,0],':model-warnings',False,':testing-function',"set-testing-traces"])


def run_it():
    sys.stdout = tester.out_file
    bst_ppm.experiment(1)
    sys.stdout = sys.__stdout__    
