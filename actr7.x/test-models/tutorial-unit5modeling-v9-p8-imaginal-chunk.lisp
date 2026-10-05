
(ssp :starting-parameters (:model-warnings nil :testing-function set-testing-traces))

(actr-load "ACT-R:test-models;models;declarative-issues-v9.lisp")


(defun run-it ()
  (sgp :v nil :ncnar nil)
  (run 27.48)
  (sgp :v t :trace-detail high)
  (buffer-chunk imaginal))