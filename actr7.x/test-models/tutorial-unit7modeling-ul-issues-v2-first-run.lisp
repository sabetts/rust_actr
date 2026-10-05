
(actr-load "ACT-R:tutorial;lisp;ul-issues.lisp")
(actr-load "ACT-R:test-models;models;utility-learning-issues-model-v2.lisp")

(defun run-it () 
  (sgp :v nil)
  (run 7)
  (sgp :v t)
  (run 14))