"""Production history grid and graph viewers."""

from __future__ import annotations

from pathlib import Path

from harness import Context, Scenario

COUNT = Path(__file__).resolve().parents[3] / "fixtures" / "models" / "tutorial" / "unit1" / "count.lisp"


def production_history(ctx: Context) -> None:
    loaded = ctx.load_model(COUNT)
    assert loaded and str(loaded[0]) == "1", f"load failed: {loaded}"
    ctx.set_global("global_current_model_name", "COUNT")

    ctx.source_dialog("60-ctrl-panel-recorded-label.tcl")
    ctx.source_dialog("67-p-history-button.tcl")
    ctx.source_dialog("67a-ptrace-button.tcl")
    ctx.source_dialog("68-pgraph-button.tcl")

    ctx.eval("select_ptrace")
    ptrace = ctx.find_window(".ptrace")
    ctx.eval("select_pgraph")
    pgraph = ctx.find_window(".pgraph")

    ran = ctx.run_model()
    assert ran and str(ran[0]) == "1", f"run failed: {ran}"
    assert int(ctx.values(ran[1])[1]) > 0, f"run produced no events: {ran}"

    # Production grid.
    ctx.invoke(f"{ptrace}.get")
    grid_canvas = f"{ptrace}.frame.canvas"
    items = ctx.canvas_items(grid_canvas)
    assert items, "production grid canvas is empty"
    assert ctx.canvas_items(grid_canvas, "zoom"), "production grid has no boxes"
    assert ctx.canvas_items(f"{ptrace}.frame.canvas1"), (
        "production grid has no row labels"
    )

    # Production graph, including each processor mode.
    ctx.invoke(f"{pgraph}.get")
    graph_canvas = f"{pgraph}.frame.f.canvas"
    assert ctx.canvas_items(graph_canvas), "production graph canvas is empty"
    for i in range(1, 7):
        ctx.invoke(f"{pgraph}.buttons.grid{i}")
        ctx.update()
        assert ctx.canvas_items(graph_canvas), f"graph mode {i} produced no items"


SCENARIOS = [
    Scenario("production-history", production_history, tags=("production", "graph")),
]
