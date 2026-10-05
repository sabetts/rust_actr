import importlib
import os
import sys
import glob

import general_import


# call this first on the ACT-R side (turn-off-act-r-output)
# Then run tester.run_tests()



actr = general_import.import_path('../tutorial/python/actr.py')


actr.load_act_r_model('ACT-R:test-models;model-tester.cl')


def ignore_real_time(time,real_time=False):
    return actr.current_connection.evaluate("run", time, False)

actr.run = ignore_real_time

def ignore_real_window(title,visible=True,width=300,height=300,x=300,y=300):
    return actr.current_connection.evaluate_single("open-exp-window",title,[["visible", False],["width", width],
                                                                            ["height", height],["x", x], ["y", y]])

actr.open_exp_window = ignore_real_window

out_file = ''

def record_test_output(string):
    if actr.current_connection.interface.show_output:
        out_file.write(string)

actr.add_command("Python-model-tester-output",record_test_output,"Internal command for saving model output to a file for testing. Do not call.")

def set_testing_traces():
    actr.set_parameter_value(":v",True)
    actr.set_parameter_value(":cmdt",True)

actr.add_command("python-set-testing-traces",set_testing_traces,"Internal command turning on output trace parameters. Do not call.")


def run_tests():
    mismatch = []
    for f in glob.glob('./*.py'):
        path, filename = os.path.split(f)
        filename, ext = os.path.splitext(filename)

        if (filename != 'tester' and filename != 'general_import' and filename != 'tutorial_timing'):
            if run_model(filename):
                mismatch.append(filename)
    return mismatch

  
def run_model(file_name):
    global out_file

    print("\n###################################\nTesting file %s\n"%file_name)
    
    if (file_name[-5:] == '_test'):
       output = file_name[:-5]
    else:
       output = file_name

    actr.set_system_parameter_value(':starting-parameters',[':testing-function',"'python-set-testing-traces'"])

    with open("./"+output+".out","w") as f:

        out_file = f

        m = __import__(file_name)
        importlib.reload(m)

        actr.stop_output()
    
        actr.monitor_command("model-trace","Python-model-tester-output")
        actr.monitor_command("command-trace","Python-model-tester-output")
        actr.monitor_command("warning-trace","Python-model-tester-output")
        actr.monitor_command("general-trace","Python-model-tester-output")

        m.run_it()

        actr.set_system_parameter_value(':starting-parameters',None)

        actr.remove_command_monitor("model-trace","Python-model-tester-output")
        actr.remove_command_monitor("command-trace","Python-model-tester-output")
        actr.remove_command_monitor("warning-trace","Python-model-tester-output")
        actr.remove_command_monitor("general-trace","Python-model-tester-output")

        actr.resume_output()

    return actr.current_connection.evaluate_single("testing-model-compare",output)


