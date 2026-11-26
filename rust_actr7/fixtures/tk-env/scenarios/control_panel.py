"""Control panel: load, run, reset, reload, stop, and model menu updates."""

from __future__ import annotations

from harness import Context, Scenario

PANEL = ".control_panel.frame.c.contents"


def control_panel(ctx: Context) -> None:
    # The real control-panel pieces, in the same order the Tk Environment
    # sources them.  06 also installs the creating/deleting-model monitors.
    for name in (
        "10-ctrl-panel-model-label.tcl",
        "20-ctrl-panel-running-label.tcl",
        "06-ctrl-panel-show-loaded.tcl",
        "11-ctrl-panel-load-button.tcl",
        "22-ctrl-panel-reset-buttons.tcl",
        "23-ctrl-panel-run-button.tcl",
        "24-ctrl-panel-stop-button.tcx",
    ):
        print(f"sourcing {name}", flush=True)
        ctx.source_dialog(name)

    print("checking initial model state", flush=True)
    assert ctx.get_global("global_current_model_name") == "No Current Model"
    assert ctx.eval("currently_selected_model") == "nil"

    print("loading model", flush=True)
    ctx.clear_message_boxes()
    # Invoke the real Load button: it sets the global `fname` that
    # load_model_file's deferred after-script relies on.
    ctx.set_global("::tk_open_file", ctx.model.as_posix())
    ctx.invoke(f"{PANEL}.load")
    print("load returned", flush=True)
    ctx.update()
    assert ctx.eval("currently_selected_model") == "ADDITION"
    assert ctx.current_model() == "ADDITION"
    assert not ctx.message_boxes(), ctx.message_boxes()

    # Run goes through the real button proc (string run time, warning monitor).
    ctx.set_global("run_button_time", ctx.run_time)
    before = int(ctx.call_act_r_command("mp-time-ms")[0])
    ctx.invoke(f"{PANEL}.run_frame.run")
    ctx.update()
    after = int(ctx.call_act_r_command("mp-time-ms")[0])
    assert after > before, f"run did not advance time: {before} -> {after}"
    assert not ctx.message_boxes(), ctx.message_boxes()

    # Reset returns the model clock to zero.
    ctx.invoke(f"{PANEL}.r_frame.reset")
    ctx.update()
    assert int(ctx.call_act_r_command("mp-time-ms")[0]) == 0
    assert ctx.current_model() == "ADDITION"

    # Reload uses the recorded model file and keeps the model selected.
    ctx.invoke(f"{PANEL}.r_frame.reload")
    ctx.update()
    assert ctx.eval("currently_selected_model") == "ADDITION"
    assert ctx.current_model() == "ADDITION"

    # Stop is safe to invoke when nothing is running.
    ctx.eval("stop_model")
    ctx.update()


SCENARIOS = [
    Scenario(
        "control-panel", control_panel, tags=("control", "lifecycle"), timeout=30.0
    ),
]
