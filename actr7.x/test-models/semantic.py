import actr

def run_it():
    
    actr.run(10)
    actr.goal_focus(['ISA', 'is-member', 'object', 'canary', 'category', 'animal'])
    actr.run(10)
    actr.goal_focus(['ISA', 'is-member', 'object', 'canary', 'category', 'fish'])
    actr.run(10)

actr.load_act_r_model("ACT-R:tutorial;unit1;semantic.lisp")
