"""Dispatcher status: connections, per-connection commands, and docs."""

from __future__ import annotations

from harness import Context, Scenario


def dispatcher(ctx: Context) -> None:
    # Give this connection a documented command so the per-connection command
    # list and documentation pane have something to show.
    ctx.eval('proc probe_cmd {args} {}')
    ctx.eval('add_cmd "probe-cmd" "probe_cmd" "probe documentation"')

    ctx.source_dialog("91-dispatcher-status.tcl")
    ctx.eval("make_dispatcher_status_viewer")

    ctx.invoke(".dispatcher.get")
    names = ctx.list_items(".dispatcher.list_frame_1.list_box")
    assert names, "dispatcher status listed no connections"
    assert any(str(n) == "ACT-R Environment" for n in names), names

    ctx.tcl.call(".dispatcher.list_frame_1.list_box", "selection", "set", "0")
    ctx.eval("select_dispatcher_name")
    commands = ctx.list_items(".dispatcher.list_frame.list_box")
    assert commands, "selected connection listed no commands"
    assert any(str(c) == "probe-cmd" for c in commands), commands

    ctx.tcl.call(".dispatcher.list_frame.list_box", "selection", "set", "0")
    ctx.eval("select_command_name")
    doc = ctx.get_global(".dispatcher.d2.value")
    assert doc.strip(), "selected command had no documentation"
    # The address/counter labels should have been filled from list-connections.
    assert ctx.get_global(".dispatcher.li0.value").strip()


SCENARIOS = [
    Scenario("dispatcher", dispatcher, tags=("dispatcher", "commands")),
]
