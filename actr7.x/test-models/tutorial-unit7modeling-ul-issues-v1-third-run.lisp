
(actr-load "ACT-R:tutorial;lisp;ul-issues.lisp")
(actr-load "ACT-R:test-models;models;utility-learning-issues-model-v1.lisp")

(defun run-it () 
  (sgp :v nil)
  (run 14)
  (sgp :v t)
  (run 7))