import pathlib
import sys
import unittest

ROOT = pathlib.Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "examples"))

import lunar_lander  # noqa: E402


class LunarExampleTests(unittest.TestCase):
    def test_model_mode_reaches_a_terminal_state(self):
        lander = lunar_lander.LunarLander(1)
        model = lunar_lander.build_model(lander, 1, trace=False)
        model.run(20.0)
        self.assertIn(lander.status, {"landed", "crashed"})
        self.assertLessEqual(lander.elapsed, 20.0)
        self.assertGreaterEqual(lander.fuel, 0.0)

    def test_human_physics_starts_on_thrust(self):
        lander = lunar_lander.LunarLander(2)
        self.assertEqual(lander.status, "ready")
        lander.set_thrusters(True, False, False)
        self.assertEqual(lander.status, "flying")
        fuel = lander.fuel
        lander.step(0.05)
        self.assertLess(lander.fuel, fuel)


if __name__ == "__main__":
    unittest.main()
