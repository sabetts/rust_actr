"""Stepper UI and event queue against the stepper fixture model."""

from __future__ import annotations

from pathlib import Path

from harness import Context, Scenario

STEPPER_MODEL = Path(__file__).resolve().parents[3] / "fixtures" / "stepper" / "stepper-model.lisp"


def stepper(ctx: Context) -> None:
    loaded = ctx.load_model(STEPPER_MODEL)
    assert loaded and str(loaded[0]) == "1", f"load failed: {loaded}"
    ctx.set_global("global_current_model_name", "STEPPER-FIXTURE")

    ctx.source_dialog("21-stepper.tcl")
    ctx.source_dialog("21a-event-queue.tcl")

    # Start the real stepper tool (creates .stepper and calls start-stepper).
    ctx.eval("select_stepper")
    assert ctx.eval("winfo exists .stepper") == "1", "stepper window was not created"
    stepper_id = ctx.get_global("current_stepper_id")
    assert stepper_id and stepper_id != "-1", stepper_id

    # Record pauses and automatically press Step a few times, then Stop.
    ctx.set_global("::stepper_seen", "")
    ctx.set_global("::auto_steps", "3")
    ctx.eval("rename wait_for_stepper __real_wait_for_stepper")
    ctx.eval(
        """
proc wait_for_stepper {model event} {
  __real_wait_for_stepper $model $event
  lappend ::stepper_seen $event
  if {$::auto_steps > 0} {
    incr ::auto_steps -1
    after 5 {catch {stepper_step_button}}
  } else {
    after 5 {catch {stepper_stop_button}}
  }
}
"""
    )

    ctx.run_async("run", "nil", 2.0)
    result = ctx.wait_async(timeout=45.0)
    assert result and str(result[0]) == "1", f"stepper run failed: {result}"
    assert result and str(result[0]) == "1", f"stepper run failed: {result}"
    assert not ctx.message_boxes(), ctx.message_boxes()

    seen = ctx.values(ctx.getvar("::stepper_seen"))
    assert seen, "stepper produced no pause events"
    assert any("SET-BUFFER-CHUNK GOAL" in e for e in seen), seen
    assert any("CONFLICT-RESOLUTION" in e for e in seen), seen
    assert str(ctx.getvar("stepper_tutored")) == "0"
    assert ctx.eval("winfo exists .stepper") == "1"

    # Stop and close the stepper tool.
    ctx.eval("stepper_stop_button")
    ctx.call_act_r_command("stop-stepper", "nil", stepper_id)
    ctx.eval("destroy .stepper")

    # Event queue uses the real periodic update script.
    ctx.invoke(".control_panel.frame.c.contents.event_queue_button")
    ctx.run_updates()
    queue = ctx.text(".event_queue.frame.text")
    assert "Events in the queue" in queue, queue[:300]


SCENARIOS = [
    Scenario("stepper", stepper, tags=("stepper", "event-queue")),
]
