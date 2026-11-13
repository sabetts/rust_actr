import unittest

import rust_actr as actr


class Counter(actr.Module):
    version = "0.1"
    documentation = "Python test module"
    step = 1

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
        return name == "state" and value == "free"

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


if __name__ == "__main__":
    unittest.main()
