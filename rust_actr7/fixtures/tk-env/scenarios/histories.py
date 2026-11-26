"""Retrieval and buffer history viewers against a real addition run."""

from __future__ import annotations

from harness import Context, Scenario


def histories(ctx: Context) -> None:
    loaded = ctx.load_model()
    assert loaded and str(loaded[0]) == "1", f"load failed: {loaded}"
    ctx.set_global("global_current_model_name", "ADDITION")

    ctx.source_dialog("60-ctrl-panel-recorded-label.tcl")
    ctx.source_dialog("63-retrieval-history.tcl")
    ctx.source_dialog("64-buffer-history.tcl")

    ret = ctx.eval("make_declarative_history_viewer")
    buf = ctx.eval("make_buffer_history_viewer")

    ran = ctx.run_model()
    assert ran and str(ran[0]) == "1", f"run failed: {ran}"
    assert int(ctx.values(ran[1])[1]) > 0, f"run produced no events: {ran}"

    # Retrieval history: times -> matching chunks -> details/request/activation.
    ctx.invoke(f"{ret}.get")
    times = ctx.list_items(f"{ret}.list_frame_1.list_box")
    assert times, "retrieval history listed no times"
    assert ctx.getvar(f"{ret}.l0.value").startswith("Retrieval History Data")
    lb1 = f"{ret}.list_frame_1.list_box"
    lb2 = f"{ret}.list_frame.list_box"
    tb1 = f"{ret}.text_frame_1.text"
    tb2 = f"{ret}.text_frame_2.text"
    tb3 = f"{ret}.text_frame_3.text"
    ctx.tcl.call(lb1, "selection", "set", "0")
    ctx.tcl.call(
        "select_dm_history_time",
        lb1,
        lb2,
        tb1,
        tb2,
        tb3,
        f"{ret}.value",
        f"{ret}.value2",
    )
    chunks = ctx.list_items(lb2)
    assert chunks, "retrieval history listed no matching chunks"
    assert ctx.text(tb2).strip(), "retrieval request pane was empty"
    ctx.tcl.call(lb2, "selection", "set", "0")
    ctx.tcl.call("select_dm_history_chunk", lb2, tb1, tb3, f"{ret}.value2")
    assert ctx.text(tb1).strip(), "retrieval details pane was empty"

    # Buffer history: times -> buffer actions -> before/after panes.
    ctx.invoke(f"{buf}.get")
    btimes = ctx.list_items(f"{buf}.list_frame_1.list_box")
    assert btimes, "buffer history listed no times"
    assert ctx.getvar(f"{buf}.l0.value").startswith("Buffer History Data")
    b1 = f"{buf}.list_frame_1.list_box"
    b2 = f"{buf}.list_frame.list_box"
    bt1 = f"{buf}.text_frame_1.text"
    bt2 = f"{buf}.text_frame_2.text"
    bt3 = f"{buf}.text_frame_3.text"
    ctx.tcl.call(b1, "selection", "set", "0")
    ctx.tcl.call(
        "select_buffer_history_time",
        b1,
        b2,
        bt1,
        bt2,
        bt3,
        f"{buf}.value",
        f"{buf}.value2",
    )
    actions = ctx.list_items(b2)
    assert actions, "buffer history listed no actions"
    ctx.tcl.call(b2, "selection", "set", "0")
    ctx.tcl.call("select_buffer_history_action", b2, bt1, bt2, bt3, f"{buf}.value2")
    assert ctx.text(bt1).strip() or ctx.text(bt2).strip() or ctx.text(bt3).strip(), (
        "buffer history detail panes were empty"
    )


SCENARIOS = [
    Scenario("histories", histories, tags=("history",)),
]
