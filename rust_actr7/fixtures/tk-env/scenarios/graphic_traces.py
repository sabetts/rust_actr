"""Horizontal and vertical graphic-trace viewers."""

from __future__ import annotations

from pathlib import Path

from harness import Context, Scenario

COUNT = Path(__file__).resolve().parents[3] / "fixtures" / "models" / "tutorial" / "unit1" / "count.lisp"


def graphic_traces(ctx: Context) -> None:
    loaded = ctx.load_model(COUNT)
    assert loaded and str(loaded[0]) == "1", f"load failed: {loaded}"
    ctx.set_global("global_current_model_name", "COUNT")

    ctx.source_dialog("60-ctrl-panel-recorded-label.tcl")
    ctx.source_dialog("71-pick-buffers.tcl")
    ctx.source_dialog("72-ctrl-panel-gt-button.tcl")
    ctx.source_dialog("72a-ctrl-panel-gt-horizontal.tcl")
    ctx.source_dialog("73-ctrl-panel-gt-vertical.tcl")

    ctx.eval("select_graphic_trace")
    horiz = ctx.find_window(".graphic_trace")
    ran = ctx.run_model()
    assert ran and str(ran[0]) == "1", f"run failed: {ran}"
    ctx.invoke(f"{horiz}.get")
    ctx.update()
    assert ctx.canvas_items(f"{horiz}.frame.canvas"), "horizontal trace canvas empty"
    assert ctx.canvas_items(f"{horiz}.canvas1"), "horizontal trace buffer column empty"

    ctx.eval("select_vert_graphic_trace")
    windows = [
        w
        for w in ctx.values(ctx.tcl.call("winfo", "children", "."))
        if w.startswith(".graphic_trace")
    ]
    verts = [w for w in windows if w != horiz]
    assert verts, f"vertical trace window not found: {windows}"
    vert = verts[-1]
    ctx.invoke(f"{vert}.get")
    ctx.update()
    assert ctx.canvas_items(f"{vert}.frame.canvas"), "vertical trace canvas empty"


SCENARIOS = [
    Scenario("graphic-traces", graphic_traces, tags=("graphic-trace", "smoke")),
]
