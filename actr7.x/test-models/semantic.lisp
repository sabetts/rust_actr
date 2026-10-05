

(defun run-it ()
  
  (run 10)
  (goal-focus (ISA is-member object canary category animal))
  (run 10)
  (goal-focus (ISA is-member object canary category fish))
  (run 10))

(load-act-r-model "ACT-R:tutorial;unit1;semantic.lisp")
