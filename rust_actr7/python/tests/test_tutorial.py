import os
import pathlib
import unittest

HERE = pathlib.Path(__file__).resolve()
REPO = HERE.parents[3]
os.environ.setdefault("ACTR_DIR", str(REPO / "actr7.x"))

import rust_actr.tutorial as tutorial  # noqa: E402
from rust_actr.tutorial import runner  # noqa: E402


class TutorialTests(unittest.TestCase):
    def test_all_models_compile(self):
        self.assertGreaterEqual(len(tutorial.available()), 25)
        for path in tutorial.available():
            with self.subTest(path=path):
                model = tutorial.build_model(path)
                model._ensure_session()

    def test_count_runs(self):
        import rust_actr.actr as actr

        actr.load_act_r_model("ACT-R:tutorial;unit1;count.lisp")
        actr.run(10)
        self.assertIn("PRODUCTION-FIRED INCREMENT", actr.current_model().trace)

    def test_demo2_task(self):
        result = runner.run_task("demo2", "experiment")
        self.assertIn(result.upper(), "BCDFGHJKLMNPQRSTVWXYZ")

    def test_fan_no_pm_sentence(self):
        module = runner.load_task("fan_no_pm")
        seconds, correct = module.sentence("'lawyer'", "'store'", True, "person")
        self.assertTrue(correct)
        self.assertGreater(seconds, 0)

    def test_grouped_recall(self):
        result = runner.run_task("grouped", "recall")
        self.assertEqual(len(result), 8)
        self.assertEqual(sorted(result), ["1", "2", "3", "4", "5", "6", "7", "8"])

    def test_onehit_hands(self):
        result = runner.run_task("onehit", "hands", 2)
        self.assertEqual(len(result), 4)


if __name__ == "__main__":
    unittest.main()
