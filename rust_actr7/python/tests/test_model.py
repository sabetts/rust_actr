import unittest

import rust_actr as actr


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
        self.assertIn('(call-act-r-command "create-example-memories")', source)
        self.assertIn("(set-buffer-chunk 'imaginal '(isa chunk category unknown))", source)
        self.assertIn('"Sentence Experiment"', source)
        model.run(0.0)

    def test_raw_form(self):
        model = actr.Model("raw")
        model.chunk_type("goal", "state")
        model.form("(when t (add-dm (start isa chunk)))")
        self.assertIn("(when t (add-dm (start isa chunk)))", model.to_lisp())
        model.run(0.0)


if __name__ == "__main__":
    unittest.main()
