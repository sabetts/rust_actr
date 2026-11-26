"""Experiment windows and AGI callbacks through the Tk Environment code."""

from __future__ import annotations

from pathlib import Path

from harness import Context, Scenario

COUNT = Path(__file__).resolve().parents[3] / "fixtures" / "models" / "tutorial" / "unit1" / "count.lisp"


def experiment_windows(ctx: Context) -> None:
    loaded = ctx.load_model(COUNT)
    assert loaded and str(loaded[0]) == "1", f"load failed: {loaded}"

    # This registers the client window handler and creates .env_window.
    ctx.source_dialog("99-exp-window.tcl")
    assert ctx.eval('name_to_window "WIN"') == ""

    opened = ctx.call_act_r_command("open-exp-window", "nil", "WIN")
    assert opened and str(opened[0]) == "window", opened
    win = ctx.eval('name_to_window "WIN"')
    assert win, "open callback did not create the Tk window"
    assert ctx.eval(f"winfo exists {win}") == "1"

    text_key = ctx.call_act_r_command(
        "add-text-to-exp-window", "nil", "WIN", "hello"
    )[0]
    line_key = ctx.call_act_r_command(
        "add-line-to-exp-window",
        "nil",
        "WIN",
        ctx.tcl_list("10", "10"),
        ctx.tcl_list("100", "100"),
        "black",
    )[0]
    ctx.update()
    canvas = f"{win}.can"
    texts = ctx.canvas_items(canvas, "text")
    lines = ctx.canvas_items(canvas, "line")
    assert ctx.canvas_items(canvas, str(text_key)), f"text tag missing: {text_key}"
    assert any("hello" in ctx.canvas_text(canvas, i) for i in texts)
    assert ctx.canvas_items(canvas, str(line_key)), f"line tag missing: {line_key}"

    # A button with a client-owned action; clicking it must invoke the proc.
    # Used later once complex nested options are sent as real JSON; Tcl's
    # flat list2json cannot represent an options list.

    # Key presses are forwarded through output-key monitors.
    ctx.eval("set ::key_seen 0")
    ctx.eval("proc key_handler {args} {set ::key_seen 1}")
    ctx.eval('add_cmd "key-handler" "key_handler" "test key monitor"')
    ctx.send("monitor", "output-key", "key-handler", "simple")
    ctx.eval(f"event generate {win} <KeyPress> -keysym a")
    ctx.update()
    assert ctx.getvar("::key_seen") == 1, "key press did not reach output-key"

    # Close callback destroys the Tk window and its name mapping.
    closed = ctx.call_act_r_command("close-exp-window", "nil", "WIN")
    assert closed and str(closed[0]).lower() == "true", closed
    ctx.update()
    assert ctx.eval('name_to_window "WIN"') == ""
    assert ctx.eval(f"winfo exists {win}") != "1"


SCENARIOS = [
    Scenario("experiment-windows", experiment_windows, tags=("windows", "agi")),
]
