import unittest

import rust_actr as actr
import rust_actr.actr as client


class ModelTests(unittest.TestCase):
    def make_model(self):
        model = actr.Model("py-test", seed=(100, 0))
        model.chunk_type("goal", "state")
        start = model.add_chunk("start", "goal", state="start")
        model.goal_focus(start)
        model.add_production(
            """
(p finish
   =goal>
     state start
   ==>
   =goal>
     state done)
"""
        )
        return model

    def test_build_and_run(self):
        model = self.make_model()
        trace = model.run(1.0)

        self.assertIn("PRODUCTION-FIRED FINISH", trace)
        self.assertIn("Stopped because no events left to process", trace)
        self.assertIn("PRODUCTION-FIRED FINISH", model.trace)
        self.assertGreater(model.time, 0.0)
        self.assertTrue(model.compiled)

    def test_generated_source(self):
        model = self.make_model()
        source = model.to_lisp()

        self.assertIn("(define-model py-test", source)
        self.assertIn("(chunk-type goal state)", source)
        self.assertIn("(add-dm (start isa goal state start))", source)
        self.assertIn("(goal-focus start)", source)
        self.assertIn("(p finish", source)
        self.assertIn("==>", source)

    def test_values_and_slot_names(self):
        model = actr.Model("values")
        model.chunk_type("number", "vocal_rep", defaults={"vocal_rep": actr.String("zero")})
        model.add_chunk(
            "zero",
            "number",
            vocal_rep=actr.String("zero"),
            enabled=True,
            missing=None,
        )
        source = model.to_lisp()
        self.assertIn('(chunk-type number (vocal-rep "zero"))', source)
        self.assertIn('vocal-rep "zero"', source)
        self.assertIn("enabled t", source)
        self.assertIn("missing nil", source)

    def test_production_source_is_preserved(self):
        model = actr.Model("preserve")
        production = '(p say\n  =goal> state start\n  ==>\n  !output! ("a\\nb"))'
        model.add_production(production)

        self.assertIn(production, model.to_lisp())

    def test_repeated_runs_and_take_trace(self):
        model = self.make_model()
        first = model.run(1.0)
        second = model.run(1.0)

        self.assertIn("PRODUCTION-FIRED FINISH", first)
        self.assertNotIn("PRODUCTION-FIRED FINISH", second)
        all_trace = model.take_trace()
        self.assertIn("PRODUCTION-FIRED FINISH", all_trace)
        self.assertEqual(model.trace, "")

    def test_reset_rearms_initial_goal(self):
        model = self.make_model()
        model.run(1.0)
        model.take_trace()
        model.reset()
        reset_trace = model.run(0.0)

        self.assertIn("SET-BUFFER-CHUNK GOAL START", reset_trace)
        self.assertNotIn("PRODUCTION-FIRED FINISH", reset_trace)

    def test_definitions_freeze_after_run(self):
        model = self.make_model()
        model.run(0.1)
        with self.assertRaises(RuntimeError):
            model.add_chunk("later", "goal", state="later")

    def test_invalid_production_reports_source(self):
        model = actr.Model("broken")
        model.chunk_type("goal", "state")
        model.add_chunk("start", "goal", state="start")
        model.goal_focus("start")
        model.add_production("(p broken =goal> state =x ==> =goal> state =y")
        with self.assertRaises(RuntimeError) as caught:
            model.run(0.1)
        self.assertIn("could not compile model", str(caught.exception))
        self.assertIn("(define-model broken", str(caught.exception))

    def test_rejects_non_production_text(self):
        model = actr.Model("bad")
        with self.assertRaises(ValueError):
            model.add_production("=goal> state start ==> =goal> state done")

    def test_goal_focus_requires_dm_chunk(self):
        model = actr.Model("bad")
        with self.assertRaises(ValueError):
            model.goal_focus("missing")

    def test_inline_goal_focus(self):
        model = actr.Model("inline")
        model.chunk_type("goal", "state")
        model.goal_focus(isa="goal", state="start")
        self.assertIn("(goal-focus (isa goal state start))", model.to_lisp())
        self.assertIn("SET-BUFFER-CHUNK GOAL (ISA GOAL STATE START)", model.run(0.0))

    def test_tutorial_model_body_forms(self):
        model = actr.Model("forms")
        model.chunk_type("goal", "state")
        model.add_chunk("shark")
        model.add_dm("dangerous")
        model.define_chunks("unknown")
        model.set_similarities([("small", "medium", -0.4), ("medium", "large", -0.4)])
        model.set_base_levels([("f00", 5), ("f01", 4, 1.0)])
        model.set_all_base_levels(3)
        model.spp("p1", u=-2, reward=1.0)
        model.set_visloc_default(attended=False)
        model.declare_buffer_usage("goal", "goal", "state")
        model.call_act_r_command("create-example-memories")
        model.set_initial_buffer("imaginal", isa="chunk", category="unknown")
        model.install_device("Sentence Experiment", visible=False, width=600, height=300)
        model.add_production("(p p1 =goal> state start ==> =goal> state done)")
        model.declare_buffer_usage("imaginal", "goal", all_slots=True)

        source = model.to_lisp()
        self.assertIn("(add-dm (shark))", source)
        self.assertIn("(add-dm (dangerous))", source)
        self.assertIn("(define-chunks (unknown))", source)
        self.assertIn("(set-similarities (small medium -0.4) (medium large -0.4))", source)
        self.assertIn("(set-base-levels (f00 5) (f01 4 1.0))", source)
        self.assertIn("(set-all-base-levels 3)", source)
        self.assertIn("(spp p1 :u -2 :reward 1.0)", source)
        self.assertIn("(set-visloc-default attended nil)", source)
        self.assertIn("(declare-buffer-usage goal goal state)", source)
        self.assertIn("(declare-buffer-usage imaginal goal :all)", source)
        self.assertIn('(call-act-r-command "create-example-memories")', source)
        self.assertIn("(set-buffer-chunk 'imaginal '(isa chunk category unknown))", source)
        self.assertIn('"Sentence Experiment"', source)
        model.run(0.0)

    def test_sgp_accepts_key_without_colon(self):
        model = actr.Model("sgp-keys")
        model.sgp("lf", 0.05)
        model.sgp(":esc", True)
        self.assertEqual(model._sgp, [(":lf", 0.05), (":esc", True)])

    def test_runtime_parameter_key_without_colon(self):
        model = self.make_model()
        model.set_parameter_value("lf", 0.2)
        self.assertAlmostEqual(float(model.get_parameter_value("lf")), 0.2)
        self.assertEqual(
            model.get_parameter_value("lf"), model.get_parameter_value(":lf")
        )

    def test_runtime_callbacks(self):
        seen = []
        model = actr.Model("callback", seed=(100, 0))
        model.chunk_type("goal", "state")
        model.goal_focus(isa="goal", state="start")
        model.add_production(
            """
(p record
   =goal> state start
   ==>
   =goal> state done
   !eval! ("record" 42))
"""
        )
        model.add_command("record", lambda value: seen.append(value))
        model.run(1.0)

        self.assertEqual(seen, ["42"])
        self.assertEqual(model.buffer_slot_value("goal", "state"), "DONE")
        self.assertIn("PRODUCTION-FIRED RECORD", model.trace)

    def test_runtime_buffers_and_dm(self):
        model = actr.Model("runtime", seed=(100, 0))
        model.chunk_type("goal", "state")
        model.chunk_type("note", "value")
        model.goal_focus(isa="goal", state="start")
        model.run(0.0)

        model.add_dm(actr.Chunk("n1", "note", {"value": "hello"}))
        model.set_buffer_chunk("goal", ["isa", "note", "value", "hello"])
        self.assertEqual(model.buffer_slot_value("goal", "value"), "HELLO")
        self.assertEqual(model.chunk_slot_value("n1", "value"), "HELLO")
        model.goal_focus(isa="goal", state="restarted")
        model.run(0.0)
        self.assertEqual(model.buffer_slot_value("goal", "state"), "RESTARTED")

    def test_device_and_output_key(self):
        model = actr.Model("demo", seed=(100, 0))
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
        keys = []
        model.add_command("key", lambda model_name, key: keys.append(key))
        model.monitor_command("output-key", "key")
        window = model.open_exp_window("test")
        model.add_text_to_exp_window(window, "B", x=125, y=150)
        model.install_device(window)
        model.run(10.0)

        self.assertEqual(keys, ["b"])
        self.assertIn("PRODUCTION-FIRED RESPOND", model.trace)

    def test_random_and_time(self):
        model = actr.Model("random", seed=(100, 0))
        model.chunk_type("goal", "state")
        model.goal_focus(isa="goal", state="start")
        model.run_full_time(0.25)

        self.assertEqual(model.get_time(), 250)
        self.assertIn(model.random(10), range(10))
        self.assertCountEqual(model.permute_list([1, 2, 3]), [1, 2, 3])

    def test_raw_form(self):
        model = actr.Model("raw")
        model.chunk_type("goal", "state")
        model.form("(when t (add-dm (start isa chunk)))")
        self.assertIn("(when t (add-dm (start isa chunk)))", model.to_lisp())
        model.run(0.0)


    def test_use_flushes_pending_commands(self):
        client.clear_state()
        calls = []

        client.add_command("py-pending", lambda: calls.append(True))
        model = actr.Model("pending")
        client.use(model)
        model.call_command("py-pending")

        self.assertEqual(calls, [True])
        client.clear_state()

    def test_use_binds_pending_sim_hook(self):
        client.clear_state()
        calls = []

        class FakeModel:
            sim_hook_name = "PY-HOOK"

            def add_command(self, name, function):
                calls.append(("add", name.upper()))

            def monitor_command(self, original, monitor):
                calls.append(("monitor", original.upper(), monitor.upper()))

            def set_sim_hook(self, function):
                calls.append(("hook", function))

        def hook(a, b):
            return 0.0

        client.add_command("py-hook", hook)
        client.use(FakeModel())

        self.assertIn(("add", "PY-HOOK"), calls)
        self.assertIn(("hook", hook), calls)
        client.clear_state()


if __name__ == "__main__":
    unittest.main()
