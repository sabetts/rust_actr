
(ssp :starting-parameters (:model-warnings nil :testing-function set-testing-traces))

(actr-load "ACT-R:test-models;models;declarative-issues-v9.lisp")


(defun run-it ()
  (sgp :v nil)
  (run 27.43)
  (sgp :v t)
  (run 1)
  (model-output "")
  (model-output "? (sdm - state nil)")
  (sdm - state nil)
  ; fake the python call
  (model-output "~%>>> actr.sdm('-','goal','nil')")
  (sdm - goal nil))