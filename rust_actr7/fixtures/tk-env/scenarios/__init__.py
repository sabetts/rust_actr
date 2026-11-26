"""Tk Environment scenarios.  Each module exports ``SCENARIOS``."""


def all_scenarios():
    from . import trace_history

    scenarios = []
    scenarios.extend(trace_history.SCENARIOS)
    return scenarios
