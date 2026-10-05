
(ssp :starting-parameters (:seed (1 0) :model-warnings nil :testing-function set-testing-traces))

(defun run-it () 
  (run-tests 100 50 5)
  (model-output "results: ~S~%" *results*)
  (run-tests 100 50 5 :uan t)
  (model-output "results: ~s~%" *results*))

(actr-load "ACT-R:extras;adaptive-noise;test-model.lisp")
