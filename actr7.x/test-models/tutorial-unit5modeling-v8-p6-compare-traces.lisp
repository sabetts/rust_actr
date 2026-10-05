
(ssp :starting-parameters (:model-warnings nil :testing-function set-testing-traces))

(actr-load "ACT-R:test-models;models;declarative-issues-v8.lisp")


(defun run-it ()
  (sgp :v nil)
  (run 27.15)
  (record-history "retrieval-history")
  (run-until-time 28.421)
  (sgp :v t)
  (print-chunk-activation-trace 1+2=3 27155))