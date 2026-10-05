# This is ugly...

import actr
import sys
import general_import
import tester

onehit = general_import.import_path('../tutorial/python/onehit.py',True)

actr.set_system_parameter_value(':starting-parameters',[':seed',[3,0],':testing-function',"set-testing-traces"])

seed = 100

def new_learning(n,graph=True,game=onehit.game0):

    data = [[0,0,0,0]]*20
    need_to_remove = onehit.add_key_monitor()

    for i in range(n):
        actr.reset()
        global seed
        seed = seed + 1
        actr.set_parameter_value(":v",None)
        actr.set_parameter_value(":seed",[seed,0])
        game()
        data = list(map(lambda x,y: onehit.sum_lists(x,y),data,onehit.blocks(20,5)))

    if need_to_remove:
        onehit.remove_key_monitor()

    percentages = list(map(lambda x: x[0] / n / 5,data))

    return [[sum(percentages[0:5])/5,
             sum(percentages[5:10])/5,
             sum(percentages[10:15])/5,
             sum(percentages[15:20])/5],
             percentages]



onehit.learning = new_learning

def run_it():
    sys.stdout = tester.out_file
    actr.set_parameter_value(":v",None)
    print(">>> onehit.learning(50)")
    x = onehit.learning(50)
    print(x)
    sys.stdout = sys.__stdout__    
