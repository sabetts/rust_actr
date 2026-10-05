
(ssp :starting-parameters (:model-warnings nil :testing-function set-testing-traces))

(actr-load "ACT-R:test-models;models;declarative-issues-v7.lisp")



(defun run-it ()
  (sgp :v nil)
  (run 26.541)
  (sgp :v t)
  (run-until-time 30.06))