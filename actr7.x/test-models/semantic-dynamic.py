import actr

def run_it():
    actr.goal_focus('g1')
    actr.run(10)
    actr.goal_focus('g2')
    actr.run(10)
    actr.goal_focus('g3')
    actr.run(10)
    actr.goal_focus('g2')
    actr.run(10)

actr.load_act_r_model("ACT-R:tutorial;unit8;semantic-dynamic-model.lisp")
