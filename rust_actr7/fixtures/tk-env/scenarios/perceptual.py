"""Perceptual histories and live visicon/audicon viewers."""

from __future__ import annotations

from pathlib import Path

from harness import Context, Scenario

COUNT = Path(__file__).resolve().parents[3] / "fixtures" / "models" / "tutorial" / "unit1" / "count.lisp"


def perceptual(ctx: Context) -> None:
    loaded = ctx.load_model(COUNT)
    assert loaded and str(loaded[0]) == "1", f"load failed: {loaded}"
    ctx.set_global("global_current_model_name", "COUNT")

    ctx.source_dialog("60-ctrl-panel-recorded-label.tcl")
    ctx.source_dialog("66-perceptual-history.tcl")
    ctx.source_dialog("38-visicon.tcl")
    ctx.source_dialog("39-audicon.tcl")

    # Audicon history: the viewer records on open; a tone creates data.
    aud = ctx.eval(
        'make_percept_history_viewer "Audicon" ".audicon_history" "audicon-history"'
    )
    tone = ctx.call_act_r_command("new-tone-sound", "nil", "1000", "0.1", "0.0")
    assert tone is not None
    ran = ctx.run_model("1")
    assert ran and str(ran[0]) == "1", f"run failed: {ran}"

    ctx.invoke(f"{aud}.get")
    times = ctx.list_items(f"{aud}.list_frame_1.list_box")
    assert times, "audicon history listed no times"
    label = str(ctx.getvar(f"{aud}.l3.value"))
    assert label == "Audicon", label
    ctx.tcl.call(f"{aud}.list_frame_1.list_box", "selection", "set", "0")
    ctx.eval(
        f"select_percept_history {aud}.list_frame_1.list_box "
        f"{aud}.text_frame_1.text {aud}.data"
    )
    assert ctx.text(f"{aud}.text_frame_1.text").strip(), "audicon detail pane empty"

    # Live viewers should construct and query without modal errors.
    ctx.clear_message_boxes()
    ctx.eval("select_audicon")
    aud_win = ctx.find_window(".audicon")
    assert ctx.eval(f"winfo exists {aud_win}") == "1"
    assert not ctx.message_boxes(), ctx.message_boxes()

    ctx.eval("select_visicon")
    vis_win = ctx.find_window(".visicon")
    assert ctx.eval(f"winfo exists {vis_win}") == "1"
    assert not ctx.message_boxes(), ctx.message_boxes()


SCENARIOS = [
    Scenario("perceptual", perceptual, tags=("history", "perceptual")),
]
