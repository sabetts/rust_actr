"""Python port of ACT-R:tutorial;unit1;tutor-model.lisp."""

import rust_actr as actr

m = actr.Model('tutor-model')

m.sgp(':esc', True)
m.sgp(':lf', 0.05)
m.sgp(':trace-detail', 'medium')

m.goal_focus(isa='add-pair', ten1=3, one1=6, ten2=4, one2=7)


if __name__ == "__main__":
    print(m.run(10.0), end="")
