import actr
import general_import
import tester
import sys

paired = general_import.import_path('../tutorial/python/paired.py',True)


actr.set_system_parameter_value(':starting-parameters',[':style-warnings',False,':seed',[1,0],':model-warnings',False,':testing-function',"set-testing-traces"])

actr.load_act_r_model('ACT-R:tutorial;unit8;paired-dynamic-model.lisp')

def run_it():
    sys.stdout = tester.out_file
    paired.task(2,2)
    sys.stdout = sys.__stdout__    
