import actr
import sys
import general_import
import tester

onehit = general_import.import_path('../tutorial/python/onehit.py',True)

actr.set_system_parameter_value(':starting-parameters',[':seed',[1,0],':testing-function',"set-testing-traces"])

def run_it():
    sys.stdout = tester.out_file
    actr.reset()
    actr.set_parameter_value(":v",None)
    print(">>> onehit.hands(4,True)")
    x = onehit.hands(4,True)
    print(x)
    sys.stdout = sys.__stdout__    
