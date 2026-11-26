"""Inspector viewers: declarative, procedural, buffers, and parameters."""

from __future__ import annotations

from harness import Context, Scenario

PANEL = ".control_panel.frame.c.contents"


def _contains(values, needle: str) -> bool:
    needle = needle.upper()
    return any(str(v).upper() == needle for v in values)


def viewers(ctx: Context) -> None:
    loaded = ctx.load_model()
    assert loaded and str(loaded[0]) == "1", f"load failed: {loaded}"
    # Viewer procs use the Environment's selected-model variable.
    ctx.set_global("global_current_model_name", "ADDITION")

    for name in (
        "35-declarative.tcl",
        "36-procedural.tcl",
        "37-buffer-viewer.tcl",
        "40-parameter-viewer.tcl",
    ):
        ctx.source_dialog(name)

    # Declarative viewer.
    ctx.invoke(f"{PANEL}.declarative")
    dm_win = ctx.find_window(".declarative")
    ctx.run_updates()
    chunks = ctx.list_items(f"{dm_win}.list_frame.list_box")
    assert chunks, "declarative viewer listed no chunks"
    assert _contains(chunks, "ZERO") and _contains(chunks, "ONE"), chunks
    ctx.tcl.call(f"{dm_win}.list_frame.list_box", "selection", "set", "0")
    ctx.tcl.call(
        "select_chunk",
        f"{dm_win}.list_frame.list_box",
        f"{dm_win}.text_frame.text",
        "ADDITION",
    )
    dm_text = ctx.text(f"{dm_win}.text_frame.text")
    assert "Declarative parameters" in dm_text, dm_text[:300]

    # Procedural viewer.
    ctx.invoke(f"{PANEL}.procedural")
    proc_win = ctx.find_window(".procedural")
    ctx.run_updates()
    prods = ctx.list_items(f"{proc_win}.list_frame.list_box")
    assert prods, "procedural viewer listed no productions"
    assert _contains(prods, "INITIALIZE-ADDITION"), prods
    ctx.tcl.call(f"{proc_win}.list_frame.list_box", "selection", "set", "0")
    ctx.tcl.call(
        "select_production",
        f"{proc_win}.list_frame.list_box",
        f"{proc_win}.text_frame.text",
        "ADDITION",
    )
    prod_text = ctx.text(f"{proc_win}.text_frame.text")
    assert "Parameters for production" in prod_text, prod_text[:300]

    # Buffer viewer.
    ctx.invoke(f"{PANEL}.buffers")
    buf_win = ctx.find_window(".buffers")
    ctx.run_updates()
    buffers = ctx.list_items(f"{buf_win}.list_frame.list_box")
    assert buffers, "buffer viewer listed no buffers"
    assert _contains(buffers, "GOAL"), buffers
    goal = next(i for i, b in enumerate(buffers) if str(b).upper() == "GOAL")
    ctx.tcl.call(f"{buf_win}.list_frame.list_box", "selection", "set", str(goal))
    states = ctx.values(ctx.tcl.call("info", "globals", "buffer_state_*"))
    assert states, "buffer viewer did not create a view-state variable"
    ctx.tcl.call(
        "select_buffer_viewer",
        f"{buf_win}.list_frame.list_box",
        f"{buf_win}.other_frame.text_frame.text",
        "ADDITION",
        states[-1],
    )
    buf_text = ctx.text(f"{buf_win}.other_frame.text_frame.text")
    assert buf_text.strip(), "buffer viewer produced no text for GOAL"

    # Parameter viewer.
    ctx.invoke(f"{PANEL}.param_viewer")
    par_win = ctx.find_window(".param_viewer")
    modules = ctx.list_items(f"{par_win}.list_frame_1.list_box")
    assert modules, "parameter viewer listed no modules"
    ctx.tcl.call(f"{par_win}.list_frame_1.list_box", "selection", "set", "0")
    ctx.tcl.call(
        "select_param_module",
        f"{par_win}.list_frame_1.list_box",
        f"{par_win}.list_frame.list_box",
        f"{par_win}.text_frame_1.text",
        "ADDITION",
    )
    params = ctx.list_items(f"{par_win}.list_frame.list_box")
    assert params, "parameter viewer listed no parameters for the first module"
    ctx.tcl.call(f"{par_win}.list_frame.list_box", "selection", "set", "0")
    ctx.tcl.call(
        "select_param_value",
        f"{par_win}.list_frame.list_box",
        f"{par_win}.text_frame_1.text",
        "ADDITION",
    )
    param_text = ctx.text(f"{par_win}.text_frame_1.text")
    assert param_text.strip(), "parameter viewer produced no detail text"


SCENARIOS = [
    Scenario("viewers", viewers, tags=("viewer",)),
]
