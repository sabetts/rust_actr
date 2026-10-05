
(ssp :starting-parameters (:model-warnings nil :testing-function set-testing-traces))

(actr-load "ACT-R:test-models;models;declarative-issues-v10.lisp")


(defun run-it ()
  (sgp :v nil :ncnar nil)
  (run 28)
  (sgp :v t)
  (model-output ">>> actr.sdp('old-context')")
  (sdp old-context))