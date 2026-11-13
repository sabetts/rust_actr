import unittest

import rust_actr as actr


class Counter(actr.Module):
    version = "0.1"
    documentation = "Python test module"
    step = 1
    log = []

    @classmethod
    def reset_log(cls):
        cls.log = []

    def name(self):
        return "counter"

    def buffers(self):
        return [
            actr.BufferSpec("counter", request_params=["inc"]),
        ]

    def chunk_types(self):
        return [actr.ChunkTypeSpec("counter", slots=["value"])]

    def parameters(self):
        return [actr.ParamSpec(":counter-step", default_value=self.step)]

    def reset(self, ctx, phase):
        Counter.log.append(("reset", str(phase)))

    def request(self, ctx, buffer, spec):
        inc = spec.slots.get("INC", 1)
        ctx.emit(f"REQUEST inc {inc}")
        ctx.set_busy("counter", True)
        ctx.schedule_named(50, "done", [inc * self.step])

    def on_event(self, ctx, event):
        if event.name == "done":
            value = event.payload[0]
            ctx.set_buffer("counter", "counter", {"value": value})
            ctx.set_busy("counter", False)
            ctx.emit(f"DONE {value}")

    def query(self, ctx, buffer, name, value):
        return name == "STATE" and value == "FREE"

    def notify_on_clear(self, ctx, buffer, chunk):
        Counter.log.append(("clear", buffer, chunk))

    def run_start(self, ctx):
        Counter.log.append(("run_start",))

    def run_end(self, ctx):
        Counter.log.append(("run_end",))

    def update(self, ctx, old_time, new_time):
        Counter.log.append(("update", old_time, new_time))

    def params(self, ctx, name, value=actr.UNSET):
        if name.upper() == ":COUNTER-STEP":
            if value is not actr.UNSET:
                self.step = int(value)
            return self.step
        return None


def make_model():
    model = actr.Model("py-module", modules=[Counter])
    model.chunk_type("goal", "state")
    start = model.add_chunk("start", "goal", state="start")
    model.goal_focus(start)
    model.add_production(
        """
(p fire
   =goal>
     state start
   ==>
   +counter> inc 2
   =goal>
     state done)
"""
    )
    return model


class ModuleTests(unittest.TestCase):
    def test_request_and_scheduled_event(self):
        model = make_model()
        trace = model.run(1.0)
        self.assertIn("REQUEST inc 2", trace)
        self.assertIn("DONE 2", trace)
        self.assertEqual(model.chunk_slot_value("counter", "value"), 2)

    def test_parameter_routes_to_module(self):
        model = make_model()
        model.run(0.0)
        model.set_parameter_value(":counter-step", 3)
        self.assertEqual(int(model.get_parameter_value(":counter-step")), 3)

    def test_lifecycle_hooks(self):
        Counter.reset_log()
        model = make_model()
        model.run(0.05)
        names = [entry[0] for entry in Counter.log]
        self.assertIn("run_start", names)
        self.assertIn("run_end", names)
        self.assertIn("update", names)

    def test_reset_phases(self):
        Counter.reset_log()
        model = make_model()
        model.run(0.0)
        Counter.reset_log()
        model.reset()
        resets = [entry for entry in Counter.log if entry[0] == "reset"]
        self.assertEqual(len(resets), 3)

    def test_notify_on_clear(self):
        Counter.reset_log()
        model = make_model()
        model.run(1.0)
        Counter.reset_log()
        model.clear_buffer("counter")
        clears = [entry for entry in Counter.log if entry[0] == "clear"]
        self.assertEqual(clears[0][1], "COUNTER")


if __name__ == "__main__":
    unittest.main()
