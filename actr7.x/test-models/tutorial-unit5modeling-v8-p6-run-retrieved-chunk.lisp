
(ssp :starting-parameters (:model-warnings nil :testing-function set-testing-traces))

(actr-load "ACT-R:test-models;models;declarative-issues-v8.lisp")


(defun run-it ()
  (sgp :v nil)
  (run 27.155)
  (sgp :v t)
  (run-until-time 28.421)
  (model-output "")
  (pprint-chunks 1-1=0))