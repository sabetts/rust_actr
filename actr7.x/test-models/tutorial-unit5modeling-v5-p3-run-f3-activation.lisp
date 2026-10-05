
(ssp :starting-parameters (:model-warnings nil :testing-function set-testing-traces))

(actr-load "ACT-R:test-models;models;declarative-issues-v5.lisp")


(defun run-it ()
  (sgp :v nil)
  (record-history "retrieval-history")
  (run 28)
  (sgp :v t)
  
  (print-chunk-activation-trace f3 26591))
  
