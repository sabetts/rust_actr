
(ssp :starting-parameters (:model-warnings nil :testing-function set-testing-traces))

(actr-load "ACT-R:test-models;models;declarative-issues-v9.lisp")


(defun run-it ()
  (sgp :v nil :ncnar nil)
  (run 28)
  (sgp :v t)
  (model-output "? (pprint-chunks g1 chunk0)")
  (pprint-chunks g1 chunk0))