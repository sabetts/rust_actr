
(ssp :starting-parameters (:model-warnings nil :testing-function set-testing-traces))

(actr-load "ACT-R:test-models;models;declarative-issues-v2.lisp")

(defun run-it ()
  (sgp :v nil)
  (run 10)
  (format *out-file* "Events in the queue:~%~a" (mp-queue-text)))