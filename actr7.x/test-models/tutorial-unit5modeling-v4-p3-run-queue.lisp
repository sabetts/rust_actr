
(ssp :starting-parameters (:model-warnings nil :testing-function set-testing-traces))

(actr-load "ACT-R:test-models;models;declarative-issues-v4.lisp")

(defun run-it ()

  (sgp :v nil)
  (run 30)
  (format *out-file* "Events in the queue:~%~A" (mp-queue-text)))