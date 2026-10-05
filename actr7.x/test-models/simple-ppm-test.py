import actr

def run_it():
    actr.reset()
    actr.run(1)
    actr.reset()
    actr.set_parameter_value(':ppm',1)
    actr.run(1)
    actr.reset()
    actr.set_parameter_value(':ppm',1)
    actr.set_parameter_value(':egs',1)
    actr.set_parameter_value(':seed',[100,1])
    actr.run(1)

actr.load_act_r_model("ACT-R:tutorial;unit8;simple-ppm-model.lisp")
