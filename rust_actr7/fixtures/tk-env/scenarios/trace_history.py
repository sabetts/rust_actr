"""Text Trace viewer: record, run, fetch, detail filtering, termination.

This is the scenario that reproduces the original Tk bug: the server ended an
incremental history transfer with ``[""]``, which the Tcl client's
``while {$start != ""}`` loop saw as ``{}`` and looped forever.
"""

from __future__ import annotations

from harness import Context, Scenario


def _trace_widgets(win: str) -> tuple[str, str, str, str, str]:
    return (
        f"{win}.text_frame_1.text",
        f"{win}.level_frame_1.level",
        f"{win}.min",
        f"{win}.max",
        f"{win}.l3.value",
    )


def text_trace_viewer(ctx: Context) -> None:
    ctx.source_dialog("60-ctrl-panel-recorded-label.tcl")
    ctx.source_dialog("61-trace-history.tcl")

    win = ctx.eval("make_text_trace_history_viewer")
    ctx.tcl.call("wm", "withdraw", win)
    text, level, min_entry, max_entry, label = _trace_widgets(win)

    # Fresh recording starts empty; Get History must not hang or fetch.
    ctx.tcl.call("get_trace_history_data", text, level, min_entry, max_entry, label)
    assert ctx.getvar(label) == "No Data available", ctx.getvar(label)

    loaded = ctx.call_with_errors(
        "load-act-r-code", "nil", ctx.model.as_posix(), "0"
    )
    assert loaded and str(loaded[0]) == "1", f"load failed: {loaded}"
    assert ctx.current_model().startswith("ADDITION"), ctx.current_model()

    ran = ctx.run_model()
    assert ran and str(ran[0]) == "1", f"run failed: {ran}"
    stats = ctx.values(ran[1])
    assert len(stats) >= 2, f"bad run result: {ran}"
    assert int(stats[1]) > 0, f"run produced no events: {ran}"

    ctx.tcl.call("get_trace_history_data", text, level, min_entry, max_entry, label)
    low_text = ctx.text(text)
    assert low_text, "low-detail trace history was empty"
    assert "PRODUCTION-FIRED" in low_text, low_text[:500]
    assert (
        ctx.getvar(label) == "Trace detail: low start: nil end: nil"
    ), ctx.getvar(label)

    ctx.tcl.call(f"{win}.level_frame_1.rb4", "invoke")
    ctx.tcl.call("get_trace_history_data", text, level, min_entry, max_entry, label)
    all_text = ctx.text(text)
    assert "Trace detail: ALL" in ctx.getvar(label), ctx.getvar(label)
    assert len(all_text) >= len(low_text), "ALL detail returned less than low"


SCENARIOS = [
    Scenario("trace-history", text_trace_viewer, tags=("history", "trace")),
]
