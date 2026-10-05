import actr
import sys
import general_import
import tester

onehit = general_import.import_path('../tutorial/python/onehit.py',True)

actr.set_system_parameter_value(':starting-parameters',[':seed',[3,0],':testing-function',"set-testing-traces"])

def run_it():
    sys.stdout = tester.out_file
    actr.reset()
    actr.set_parameter_value(":v",None)
    print(">>> onehit.blocks(2,5)")
    x = onehit.blocks(2,5)
    print(x)
    sys.stdout = sys.__stdout__    
