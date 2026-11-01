import ast
import pathlib
import runpy
import subprocess
import sys
import unittest

HERE = pathlib.Path(__file__).resolve()
TUTORIAL = HERE.parents[1] / "tutorial"


def run_script(relative):
    parts = relative.split()
    return subprocess.run(
        [sys.executable, str(TUTORIAL / parts[0]), *parts[1:]],
        capture_output=True,
        text=True,
        timeout=120,
    )


class TutorialTests(unittest.TestCase):
    def test_all_scripts_compile(self):
        scripts = sorted(TUTORIAL.glob("unit*/*.py"))
        self.assertGreaterEqual(len(scripts), 30)
        for path in scripts:
            with self.subTest(path=path.relative_to(TUTORIAL)):
                namespace = runpy.run_path(str(path))
                model = namespace.get("MODEL")
                if model is None:
                    model = namespace["build"]()
                model._ensure_session()

    def test_count_cli_prints_trace(self):
        result = run_script("unit1/count.py")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("PRODUCTION-FIRED INCREMENT", result.stdout)
        self.assertIn("PRODUCTION-FIRED STOP", result.stdout)

    def test_demo2_cli(self):
        result = run_script("unit2/demo2.py")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn(result.stdout.strip().upper(), "BCDFGHJKLMNPQRSTVWXYZ")

    def test_grouped_cli(self):
        result = run_script("unit5/grouped.py")
        self.assertEqual(result.returncode, 0, result.stderr)
        recalled = ast.literal_eval(result.stdout.strip())
        self.assertEqual(sorted(recalled), ["1", "2", "3", "4", "5", "6", "7", "8"])

    def test_onehit_cli(self):
        result = run_script("unit5/onehit.py hands 2")
        self.assertEqual(result.returncode, 0, result.stderr)
        scores = ast.literal_eval(result.stdout.strip())
        self.assertEqual(len(scores), 4)


if __name__ == "__main__":
    unittest.main()
