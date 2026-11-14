import pathlib
import subprocess
import sys
import unittest

HERE = pathlib.Path(__file__).resolve()
PYTHON = HERE.parents[1]
TUTORIAL = PYTHON / "tutorial"
ACTR = HERE.parents[3] / "actr7.x"


def notebook_paths():
    return [TUTORIAL / "tutorial.ipynb", *sorted(TUTORIAL.glob("unit*/unit*.ipynb"))]


class NotebookSmokeTests(unittest.TestCase):
    def test_notebooks_run(self):
        for path in notebook_paths():
            with self.subTest(notebook=path.relative_to(TUTORIAL)):
                result = subprocess.run(
                    [
                        sys.executable,
                        str(PYTHON / "tools" / "run_notebook.py"),
                        str(path),
                    ],
                    cwd=PYTHON,
                    capture_output=True,
                    text=True,
                    timeout=120,
                )
                self.assertEqual(
                    result.returncode,
                    0,
                    f"{path}\nSTDOUT:\n{result.stdout}\nSTDERR:\n{result.stderr}",
                )
                self.assertIn("OK ", result.stdout)

    @unittest.skipUnless(ACTR.is_dir(), "requires the actr7.x tutorial checkout")
    def test_generated_notebooks_are_current(self):
        result = subprocess.run(
            [sys.executable, str(PYTHON / "tools" / "port_notebooks.py"), "--check"],
            cwd=PYTHON,
            capture_output=True,
            text=True,
            timeout=120,
        )
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)


if __name__ == "__main__":
    unittest.main()
