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


if __name__ == "__main__":
    unittest.main()
