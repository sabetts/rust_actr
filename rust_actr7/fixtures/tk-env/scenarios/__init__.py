"""Tk Environment scenarios.  Each module exports ``SCENARIOS``."""


def all_scenarios():
    from . import (
        bold,
        control_panel,
        dispatcher,
        graphic_traces,
        histories,
        perceptual,
        production,
        stepper,
        trace_history,
        viewers,
        windows,
    )

    scenarios = []
    scenarios.extend(trace_history.SCENARIOS)
    scenarios.extend(control_panel.SCENARIOS)
    scenarios.extend(viewers.SCENARIOS)
    scenarios.extend(histories.SCENARIOS)
    scenarios.extend(dispatcher.SCENARIOS)
    scenarios.extend(stepper.SCENARIOS)
    scenarios.extend(windows.SCENARIOS)
    scenarios.extend(production.SCENARIOS)
    scenarios.extend(perceptual.SCENARIOS)
    scenarios.extend(bold.SCENARIOS)
    scenarios.extend(graphic_traces.SCENARIOS)
    return scenarios
