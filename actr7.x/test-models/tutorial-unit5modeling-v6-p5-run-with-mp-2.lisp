
(ssp :starting-parameters (:model-warnings nil :testing-function set-testing-traces))

(actr-load "ACT-R:test-models;models;declarative-issues-v6.lisp")


(defun run-it ()
  (sgp :mp 2 :v nil)
  (run 26.9)
  (sgp :v t :act medium)
  (run .05))