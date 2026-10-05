
(ssp :starting-parameters (:model-warnings nil :testing-function set-testing-traces))

(actr-load "ACT-R:test-models;models;declarative-issues-v8.lisp")


(defun run-it ()
  (sgp :v nil)
  (run 27.15)
  (sgp :v t :act medium :model-warnings t)
  (run-until-time 28.422))