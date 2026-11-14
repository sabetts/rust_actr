import json
import unittest

import rust_actr.actr as actr

from tutorial.unit1 import count
from tutorial.unit4 import paired
from tutorial.unit5 import grouped
from tutorial.unit7 import paired_learning_model
from tutorial.unit8 import paired_dynamic_model


class ModuleReloadTests(unittest.TestCase):
    def test_use_module_and_reload(self):
        actr.clear_state()
        actr.use_module(grouped)
        first = grouped.recall()

        self.assertIs(actr.reload(), grouped.MODEL)
        self.assertEqual(grouped.recall(), first)

    def test_reload_without_model_fails_cleanly(self):
        actr.clear_state()
        with self.assertRaises(RuntimeError):
            actr.reload()


class RebindTests(unittest.TestCase):
    def test_paired_task_uses_learning_model(self):
        actr.clear_state()
        actr.use(paired_learning_model.m)
        actr.bind_module(paired.__dict__, paired_learning_model.m)

        result = paired.task(2, 2)

        self.assertEqual(len(result), 2)
        self.assertGreater(paired_learning_model.m.time, 0)
        self.assertEqual(paired.MODEL.time, 0)

    def test_paired_task_uses_dynamic_model(self):
        actr.clear_state()
        actr.use(paired_dynamic_model.m)
        actr.bind_module(paired.__dict__, paired_dynamic_model.m)

        result = paired.task(2, 2)

        self.assertEqual(len(result), 2)
        self.assertGreater(paired_dynamic_model.m.time, 0)
        self.assertEqual(paired.MODEL.time, 0)


class HistoryTests(unittest.TestCase):
    def test_history_and_activation_helpers(self):
        actr.clear_state()
        model = count.m
        actr.use_module(count)
        model.record_history("retrieval-history")
        model.run(10)

        self.assertTrue(model.history_data_available("retrieval-history"))
        data = model.get_history_data("retrieval-history")
        self.assertIsInstance(data, str)
        self.assertIsInstance(json.loads(data), list)

        history = model.saved_activation_history()
        self.assertTrue(history)
        self.assertTrue(all(isinstance(time, int) for time, _ in history))
        self.assertTrue(any(names for _, names in history))

        model.mp_show_queue()
        model.print_dm_finsts()
        model.print_activation_trace(50)
        model.print_chunk_activation_trace("TWO", 50)


if __name__ == "__main__":
    unittest.main()
