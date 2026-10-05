
(defun run-it ()
  (goal-focus g1)
  (run 10)
  (goal-focus g2)
  (run 10)
  (goal-focus g3)
  (run 10)
  (goal-focus g2)
  (run 10))
  

(load-act-r-model "ACT-R:tutorial;unit8;semantic-dynamic-model.lisp")

