
(ssp :starting-parameters (:model-warnings nil :testing-function set-testing-traces))

(actr-load "ACT-R:test-models;models;declarative-issues-v3.lisp")

(defun run-it ()
  (sgp :act low)
  (run 12.26))