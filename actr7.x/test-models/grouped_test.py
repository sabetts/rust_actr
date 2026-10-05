import actr
import general_import

grouped = general_import.import_path('../tutorial/python/grouped.py',True)


def test_grouped():
    actr.set_parameter_value(':v',True)
    actr.set_parameter_value(':cmdt',True)
    actr.set_parameter_value(':act',True)

actr.add_command("test-grouped",test_grouped,"Testing function for grouped model. Do not call.")

actr.set_system_parameter_value(':starting-parameters',[':testing-function',"'test-grouped'"])


def run_it():
    grouped.recall()
    actr.remove_command("test-grouped")

