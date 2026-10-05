""" 
The tutorial_times function runs the equivalent of the 
tutorial-total-time-results function from the tutorial-timing.cl file
but using the Python versions of the task code for comparison.

def total_times(name,fib=False):

name is the file name for a .csv file to save in the top-level ACT-R directory
with the timing results.
fib indicates whether or not to run the recursive fibonacci function first.

It loads the tutorial-timing.cl file into ACT-R to provide the code that
kills the traces, turns off the real-time running, and forces virtual task
windows.
"""

import importlib
import time    
import general_import


actr = general_import.import_path('../tutorial/python/actr.py')

actr.load_act_r_model('ACT-R:test-models;tutorial-timing.cl')

                                                 
def fibonacci (n):
    if n <= 1 :
        return(1)
    else:
        return(fibonacci(n-1) + fibonacci(n-2))


def model_time(unit,model,fn,params,fib=False):
    actr.set_system_parameter_value(":starting-parameters",[":seed",[101,1],":model-warnings",False])
    print("running "+model)
    if fib:
        fibonacci(41) # smaller than the Lisp version, but takes about same time
    start = time.perf_counter()    
    fn(*params)
    end = time.perf_counter()
    return([unit,model,params[0],end-start])

def just_run(n,time):
    for i in range(n):
        actr.reset()
        actr.run(time)

def run_exp(n,f,*p):
    for i in range(n):
        f(*p)   

def skip_output(*data):
    return(True)

def total_times(name,fib=False):
    data = []
    
    actr.load_act_r_model("ACT-R:tutorial;unit1;addition.lisp")
    data.append(model_time(1,"addition",just_run,[83000,10],fib))
    actr.load_act_r_model("ACT-R:tutorial;unit1;count.lisp")
    data.append(model_time(1,"count",just_run,[97000,10],fib))
    actr.load_act_r_model("ACT-R:tutorial;unit1;semantic.lisp")
    data.append(model_time(1,"semantic",just_run,[75000,10],fib))

    demo2=general_import.import_path('../tutorial/python/demo2.py',True)
    data.append(model_time(2,"demo2",run_exp,[48000,demo2.experiment],fib))
    unit2=general_import.import_path('../tutorial/python/unit2.py')
    actr.load_act_r_model('ACT-R:examples;act-r-solutions;unit-2-solution-model.lisp')
    data.append(model_time(2,"unit2",run_exp,[33000,unit2.experiment],fib))

    sperling=general_import.import_path('../tutorial/python/sperling.py',True)
    sperling.show_responses=False
    data.append(model_time(3,"sperling",sperling.experiment,[4000],fib))
    subitize=general_import.import_path('../tutorial/python/subitize.py')
    subitize.report_data=skip_output
    actr.load_act_r_model('ACT-R:examples;act-r-solutions;unit-3-subitize-solution-model.lisp')
    data.append(model_time(3,"subitize",run_exp,[2000,subitize.experiment],fib))
    
                
    paired=general_import.import_path('../tutorial/python/paired.py',True)
    data.append(model_time(4,"paired",paired.experiment,[500],fib))
    zbrodoff=general_import.import_path('../tutorial/python/zbrodoff.py',True)
    actr.load_act_r_model('ACT-R:examples;act-r-solutions;unit-4-zbrodoff-solution-model.lisp')
    data.append(model_time(4,"zbrodoff",zbrodoff.compare,[50],fib))

    fan_no_pm=general_import.import_path('../tutorial/python/fan_no_pm.py',True)
    fan_no_pm.output_person_location=skip_output
    data.append(model_time(5,"fan-no-pm",run_exp,[1400,fan_no_pm.experiment],fib))
    fan=general_import.import_path('../tutorial/python/fan.py',True)
    fan.output_person_location=skip_output
    data.append(model_time(5,"fan",run_exp,[800,fan.experiment],fib))
    siegler=general_import.import_path('../tutorial/python/siegler.py',True)
    data.append(model_time(5,"siegler",siegler.experiment,[4600],fib))
    onehit=general_import.import_path('../tutorial/python/onehit.py',True)
    actr.load_act_r_model('ACT-R:examples;act-r-solutions;unit-5-1hit-blackjack-solution-model.lisp')
    data.append(model_time(5,"one-hit",onehit.learning,[1200,False],fib))


    bst=general_import.import_path('../tutorial/python/bst.py',True)
    data.append(model_time(6,"bst",bst.experiment,[450],fib))
    unit_6_choice_solution=general_import.import_path('../examples/act-r-solutions/unit_6_choice_solution.py',True)
    actr.load_act_r_model('ACT-R:examples;act-r-solutions;unit-6-choice-solution-model.lisp')
    data.append(model_time(6,"choice",unit_6_choice_solution.data,[1700],fib))
    unit_6_choice_solution_event_based=general_import.import_path('../examples/act-r-solutions/unit_6_choice_solution_event_based.py',True)
    actr.load_act_r_model('ACT-R:examples;act-r-solutions;unit-6-choice-solution-model.lisp')
    data.append(model_time(6,"choice",unit_6_choice_solution_event_based.data,[2000],fib))

    paired=general_import.import_path('../tutorial/python/paired.py',True)
    actr.load_act_r_model('ACT-R:tutorial;unit7;paired-learning-model.lisp')
    data.append(model_time(7,"paired-learning",paired.experiment,[125],fib))
    past_tense=general_import.import_path('../tutorial/python/past_tense.py',True)
    past_tense.print_header=skip_output
    past_tense.rep_f_i=skip_output
    actr.load_act_r_model('ACT-R:examples;act-r-solutions;unit-7-past-tense-solution-model.lisp')
    data.append(model_time(7,"past-tense",run_exp,[7,past_tense.trials,20000],fib))
                
                
    bst_ppm=general_import.import_path('../tutorial/python/bst_ppm.py',True)
    data.append(model_time(8,"bst-ppm",bst_ppm.experiment,[400],fib))
    categorize=general_import.import_path('../tutorial/python/categorize.py',True)
    actr.load_act_r_model('ACT-R:examples;act-r-solutions;unit-8-categorize-solution-model.lisp')
    data.append(model_time(8,"categorize",categorize.experiment,[2800],fib))

    with open("../"+name+".csv","w") as f:

        f.write("unit,model,runs,time\n")
        for x in data:
            f.write(f'{x[0]},{x[1]},{x[2]},{x[3]}\n')


    return(data)
                                                        