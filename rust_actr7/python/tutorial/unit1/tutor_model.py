"""Python port of ACT-R:tutorial;unit1;tutor-model.lisp."""

from __future__ import annotations

import rust_actr.actr as actr

LOGICAL_PATH = 'ACT-R:tutorial;unit1;tutor-model.lisp'
MODEL_NAME = 'TUTOR-MODEL'
DEFAULT_RUN_TIME = 10.0


def build(seed=(100, 0)) -> actr.Model:
    m = actr.Model('TUTOR-MODEL', seed=seed)
    m.sgp(':ESC', True)
    m.sgp(':LF', 0.05)
    m.sgp(':TRACE-DETAIL', 'MEDIUM')
    m.goal_focus(isa='ADD-PAIR', **{'TEN1': 3, 'ONE1': 6, 'TEN2': 4, 'ONE2': 7})
    return m


def main(argv=None) -> None:
    import sys

    argv = list(sys.argv[1:] if argv is None else argv)
    seconds = float(argv[0]) if argv else DEFAULT_RUN_TIME
    print(build().run(seconds), end="")


if __name__ == "__main__":
    main()
