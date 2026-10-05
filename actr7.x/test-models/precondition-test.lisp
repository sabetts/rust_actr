

(defun run-it ()
  (run-test t)
  (run-test nil))

(defun run-test (mod)
  (reset)
  (when mod
    (goal-focus free))
  (install-device (open-exp-window ""))
  (add-text-to-exp-window nil "a")
  (run 1))
   
     
(load-act-r-model "ACT-R:test-models;models;precondition-test.lisp")
