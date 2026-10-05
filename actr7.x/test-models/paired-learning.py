import actr
import general_import

paired = general_import.import_path('../tutorial/python/paired.py',True)

alpha = .2

def test_paired():
    actr.set_parameter_value(':v',True)
    actr.set_parameter_value(':cmdt',True)
    actr.set_parameter_value(':alpha',alpha)

actr.add_command("test-paired",test_paired,"Testing function for paired model. Do not call.")

actr.set_system_parameter_value(':starting-parameters',[':seed',[1,0],':model-warnings',False,':testing-function',"'test-paired'"])

actr.load_act_r_model('ACT-R:tutorial;unit7;paired-learning-model.lisp')

def run_it():
    global alpha
    
    alpha = .2
    paired.task(2,2)
    alpha = 1.0
    paired.task(1,10)
