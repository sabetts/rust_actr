"""Tk Environment scenarios.  Each module exports ``SCENARIOS``."""


def all_scenarios():
    from . import control_panel, dispatcher, histories, trace_history, viewers

    scenarios = []
    scenarios.extend(trace_history.SCENARIOS)
    scenarios.extend(control_panel.SCENARIOS)
    scenarios.extend(viewers.SCENARIOS)
    scenarios.extend(histories.SCENARIOS)
    scenarios.extend(dispatcher.SCENARIOS)
    return scenarios
