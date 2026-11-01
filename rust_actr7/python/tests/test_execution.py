import os
import pathlib
import unittest

HERE = pathlib.Path(__file__).resolve()
REPO = HERE.parents[3]
os.environ.setdefault("ACTR_DIR", str(REPO / "actr7.x"))

import rust_actr as actr  # noqa: E402
from rust_actr.tutorial import runner  # noqa: E402


def device_model(name="device"):
    model = actr.Model(name, seed=(100, 0))
    model.sgp(":show-focus", True)
    model.chunk_type("read-letters", "step")
    model.chunk_type("array", "letter")
    model.add_dm("start")
    model.goal_focus(isa="read-letters", step="start")
    model.add_production(
        """
(p find
   =goal> ISA read-letters step start
   ==>
   +visual-location> :attended nil
   =goal> step found)
(p attend
   =goal> ISA read-letters step found
   =visual-location>
   ?visual> state free
   ==>
   +visual> cmd move-attention screen-pos =visual-location
   =goal> step attend)
(p encode
   =goal> ISA read-letters step attend
   =visual> value =letter
   ?imaginal> state free
   ==>
   =goal> step respond
   +imaginal> isa array letter =letter)
(p respond
   =goal> ISA read-letters step respond
   =imaginal> isa array letter =letter
   ?manual> state free
   ==>
   =goal> step done
   +manual> cmd press-key key =letter)
"""
    )
    return model


class ExecutionTests(unittest.TestCase):
    def test_stepped_trace_matches_fast_path(self):
        fast = actr.Model("fast", seed=(100, 0))
        fast.chunk_type("goal", "state")
        fast.goal_focus(isa="goal", state="start")
        fast.add_production("(p finish =goal> state start ==> =goal> state done)")
        slow = actr.Model("slow", seed=(100, 0))
        slow.chunk_type("goal", "state")
        slow.goal_focus(isa="goal", state="start")
        slow.add_production("(p finish =goal> state start ==> =goal> state done)")
        slow.add_command("noop", lambda: None)
        self.assertEqual(fast.run(1.0), slow.run(1.0))

    def test_callbacks_run_mid_run(self):
        model = device_model("midrun")
        seen = []
        model.add_command("key", lambda name, key: seen.append((key, model.get_time())))
        model.monitor_command("output-key", "key")
        window = model.open_exp_window("test")
        model.add_text_to_exp_window(window, "B", x=125, y=150)
        model.install_device(window)
        model.run_full_time(2.0)

        self.assertEqual(seen[0][0], "b")
        self.assertLess(seen[0][1], 2000)
        self.assertEqual(model.get_time(), 2000)

    def test_python_similarity_hook_selects_chunk(self):
        calls = []

        def hook(a, b):
            calls.append((a, b))
            try:
                return -abs(float(a) - float(b)) / 10.0
            except ValueError:
                return None

        model = actr.Model("sim", seed=(100, 0))
        model.sgp(":esc", True).sgp(":lf", 0.05).sgp(":mp", 4.0).sgp(":rt", -1.0)
        model.sgp(":ans", None)
        model.chunk_type("fto", "kind")
        model.chunk_type("goal", "state")
        model.add_chunk("t1", "fto", kind=1)
        model.add_chunk("t2", "fto", kind=2)
        model.goal_focus(isa="goal", state="start")
        model.add_production(
            "(p retrieve =goal> state start ==> =goal> state done +retrieval> isa fto kind 3)"
        )
        model.set_sim_hook(hook)
        model.run(1.0)

        self.assertTrue(calls)
        self.assertEqual(model.buffer_slot_value("retrieval", "kind"), "2")

    def test_mod_chunk(self):
        model = actr.Model("mod", seed=(100, 0))
        model.chunk_type("item", "value")
        model.add_chunk("a", "item", value=1)
        model.run(0.0)

        copy = model.copy_chunk("a")
        model.mod_chunk(copy, "value", 5)
        model.set_buffer_chunk("imaginal", copy)
        self.assertEqual(model.buffer_slot_value("imaginal", "value"), "5")

    def test_python_imaginal_action(self):
        model = actr.Model("action", seed=(100, 0))
        model.chunk_type("goal", "state")
        model.chunk_type("enc", "value")
        model.define_chunks("result")
        model.goal_focus(isa="goal", state="start")
        model.add_production(
            """
(p act
   =goal> state start
   ?imaginal-action> state free
   ==>
   =goal> state done
   +imaginal-action> action "py-action" simple t)
"""
        )
        calls = []

        def action():
            calls.append(True)
            return "result"

        model.add_command("py-action", action)
        model.run(1.0)

        self.assertEqual(calls, [True])
        self.assertIn("SET-BUFFER-CHUNK IMAGINAL RESULT", model.trace)

    def test_siegler_digit_sounds_and_speech(self):
        result = runner.run_task("siegler", "trial", 1, 1)
        self.assertIsInstance(result, str)
        self.assertNotEqual(result, "")

    def test_zbrodoff_set_runs_all_trials(self):
        result = runner.run_task("zbrodoff", "set")
        rts, counts = result
        self.assertEqual(len(rts), 3)
        self.assertEqual(counts, [8, 8, 8])


if __name__ == "__main__":
    unittest.main()
