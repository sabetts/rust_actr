
(ssp :starting-parameters (:model-warnings nil :testing-function set-testing-traces))

(actr-load "ACT-R:test-models;models;declarative-issues-v1.lisp")

(defun run-it ()
  (sgp :v nil)
  (run 2)
  (sgp-fct (list :v *out-file*))

  (sgp :rt -10)
  
  (simulate-retrieval-request - result nil simple-value-chunk t))