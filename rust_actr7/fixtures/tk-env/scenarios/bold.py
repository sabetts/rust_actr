"""BOLD graph smoke: open the viewer, record buffers, run, fetch history."""

from __future__ import annotations

from pathlib import Path

from harness import Context, Scenario

COUNT = Path(__file__).resolve().parents[3] / "fixtures" / "models" / "tutorial" / "unit1" / "count.lisp"


def bold_smoke(ctx: Context) -> None:
    loaded = ctx.load_model(COUNT)
    assert loaded and str(loaded[0]) == "1", f"load failed: {loaded}"
    ctx.set_global("global_current_model_name", "COUNT")

    ctx.source_dialog("60-ctrl-panel-recorded-label.tcl")
    ctx.source_dialog("71-pick-buffers.tcl")
    ctx.source_dialog("81-bold-button.tcl")
    ctx.source_dialog("81a-bold-browser.tcl")

    ctx.eval("make_bold_multi_graphs")
    win = ctx.find_window(".bold_graphs")
    ran = ctx.run_model()
    assert ran and str(ran[0]) == "1", f"run failed: {ran}"

    ctx.invoke(f"{win}.get")
    ctx.update()
    assert not ctx.message_boxes(), ctx.message_boxes()
    details = str(ctx.tcl.call(f"{win}.details", "cget", "-text"))
    buffers = ctx.list_items(f"{win}.list_frame.list_box")
    canvas = f"{win}.frame.canvas"
    if buffers:
        ctx.tcl.call(f"{win}.list_frame.list_box", "selection", "set", "0")
        ctx.eval(
            f"select_multi_buffer_graph {win}.list_frame.list_box {canvas} {win}"
        )
        ctx.update()
        assert ctx.canvas_items(canvas), "BOLD graph produced no canvas items"
    else:
        assert (
            "No Data" in details or "BOLD data" in details
        ), f"BOLD viewer produced nothing: {details!r}"


SCENARIOS = [
    Scenario("bold-smoke", bold_smoke, tags=("bold", "smoke")),
]
