
(defun run-it ()
  (goal-focus (ISA is-member object canary category fish))
  (sgp :v nil)
  (run-n-events 2)
  (sgp :v t)
  (run 10))

(load-act-r-model "ACT-R:tutorial;unit1;semantic.lisp")
