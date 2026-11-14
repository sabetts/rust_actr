import pathlib
import runpy
import unittest

import rust_actr.actr as client

HERE = pathlib.Path(__file__).resolve()
TUTORIAL = HERE.parents[1] / "tutorial"


def load(relative):
    client.clear_state()
    return runpy.run_path(str(TUTORIAL / relative))


def run_model(module, seconds=10.0):
    model = module.get("m") or module.get("MODEL")
    trace = model.run(seconds)
    return model, trace


class Unit1Tests(unittest.TestCase):
    def assert_model_runs(self, name):
        module = load(f"unit1/{name}.py")
        model, trace = run_model(module)
        self.assertTrue(model.compiled)
        self.assertIn("Stopped because", trace)

    def test_count(self):
        self.assert_model_runs("count")

    def test_addition(self):
        self.assert_model_runs("addition")

    def test_semantic(self):
        self.assert_model_runs("semantic")

    def test_tutor_model(self):
        self.assert_model_runs("tutor_model")

    def test_tutor_model_solution(self):
        self.assert_model_runs("tutor_model_solution")


class Unit2Tests(unittest.TestCase):
    def test_demo2_experiment(self):
        module = load("unit2/demo2.py")
        response = module["experiment"]()
        self.assertIn(str(response).upper(), "BCDFGHJKLMNPQRSTVWXYZ")

    def test_assignment_experiment(self):
        module = load("unit2/unit2.py")
        self.assertIsInstance(module["experiment"](), bool)


class Unit3Tests(unittest.TestCase):
    def test_sperling_trial(self):
        module = load("unit3/sperling.py")
        self.assertIsInstance(module["trial"](0.15), int)

    def test_sperling_experiment(self):
        module = load("unit3/sperling.py")
        self.assertIsNone(module["experiment"](1))

    def test_subitize_trial(self):
        module = load("unit3/subitize.py")
        result = module["trial"](3)
        self.assertEqual(len(result), 2)
        self.assertIsInstance(result[1], bool)

    def test_pm_issues_task(self):
        module = load("unit3/pm_issues.py")
        result = module["task"]()
        self.assertEqual(len(result), 2)
        self.assertIn(result[0], ("next", "previous"))
        self.assertIsInstance(result[1], bool)


class Unit4Tests(unittest.TestCase):
    def test_paired_task(self):
        module = load("unit4/paired.py")
        result = module["task"](2, 2)
        self.assertEqual(len(result), 2)

    def test_paired_experiment(self):
        module = load("unit4/paired.py")
        self.assertIsNone(module["experiment"](1))

    def test_zbrodoff_set(self):
        module = load("unit4/zbrodoff.py")
        rts, counts = module["set"]()
        self.assertEqual(len(rts), 3)
        self.assertEqual(counts, [8, 8, 8])

    def test_zbrodoff_block(self):
        module = load("unit4/zbrodoff.py")
        rts, counts = module["block"]()
        self.assertEqual(len(rts), 3)
        self.assertEqual(counts, [64, 64, 64])


class Unit5Tests(unittest.TestCase):
    def test_fan_sentence(self):
        module = load("unit5/fan.py")
        result = module["sentence"]("lawyer", "store", True, "person")
        self.assertEqual(len(result), 2)
        self.assertIsInstance(result[1], bool)

    def test_fan_no_pm_sentence(self):
        module = load("unit5/fan_no_pm.py")
        result = module["sentence"]("'lawyer'", "'store'", True, "person")
        self.assertEqual(len(result), 2)
        self.assertIsInstance(result[1], bool)

    def test_grouped_recall(self):
        module = load("unit5/grouped.py")
        self.assertEqual(sorted(module["recall"]()), ["1", "2", "3", "4", "5", "6", "7", "8"])

    def test_siegler_trial(self):
        module = load("unit5/siegler.py")
        self.assertIsInstance(module["trial"](1, 1), str)

    def test_onehit_hands(self):
        module = load("unit5/onehit.py")
        scores = module["hands"](2)
        self.assertEqual(len(scores), 4)

    def test_onehit_learning(self):
        module = load("unit5/onehit.py")
        block_averages, percentages = module["learning"](1, graph=False)
        self.assertEqual(len(block_averages), 4)
        self.assertEqual(len(percentages), 20)

    def test_declarative_issues(self):
        module = load("unit5/declarative_issues.py")
        _, trace = run_model(module, 2.0)
        self.assertIn("Stopped because", trace)


class Unit6Tests(unittest.TestCase):
    def test_bst_test(self):
        module = load("unit6/bst.py")
        result = module["test"](1, False)
        self.assertTrue(result)

    def test_choice_starting_model(self):
        module = load("unit6/choice.py")
        model, trace = run_model(module, 1.0)
        self.assertTrue(model.compiled)
        self.assertIn("Stopped because", trace)


class Unit7Tests(unittest.TestCase):
    def test_paired_learning_model(self):
        module = load("unit7/paired_learning_model.py")
        model, trace = run_model(module)
        self.assertTrue(model.compiled)
        self.assertIsInstance(trace, str)

    def test_past_tense_trials(self):
        module = load("unit7/past_tense.py")
        self.assertIsNone(module["trials"](1))

    def test_pcomp_issue_trial(self):
        module = load("unit7/pcomp_issues.py")
        scores, times = module["trials"](1, output=False)
        self.assertTrue(scores)
        self.assertEqual(len(scores), len(times))

    def test_ul_issues_model(self):
        module = load("unit7/ul_issues.py")
        _, trace = run_model(module, 6.95)
        self.assertIn("Stopped because", trace)


class Unit8Tests(unittest.TestCase):
    def test_bst_ppm_live_task(self):
        module = load("unit8/bst_ppm.py")
        choices = module["bst_set"](False, False, module["exp_stims"][:2])
        self.assertEqual(len(choices), 2)
        self.assertTrue(all(choice in ("over", "under") for choice in choices))

    def test_categorize_experiment(self):
        module = load("unit8/categorize.py")
        self.assertIsNone(module["experiment"](1))

    def test_simple_ppm_model(self):
        module = load("unit8/simple_ppm_model.py")
        _, trace = run_model(module, 0.5)
        self.assertIn("Stopped because", trace)

    def test_simple_dynamic_model(self):
        module = load("unit8/simple_dynamic_model.py")
        _, trace = run_model(module, 5.0)
        self.assertIn("Stopped because", trace)

    def test_semantic_dynamic_model(self):
        module = load("unit8/semantic_dynamic_model.py")
        model, trace = run_model(module, 5.0)
        self.assertIn("Stopped because", trace)

    def test_paired_dynamic_model(self):
        module = load("unit8/paired_dynamic_model.py")
        _, trace = run_model(module)
        self.assertIn("Stopped because", trace)


if __name__ == "__main__":
    unittest.main()
