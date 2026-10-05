
(ssp :starting-parameters (:seed (1 0) :model-warnings nil :testing-function set-testing-traces))

(defun run-it () 
  (model-output "without: ~s~%" (free-recall))
  (model-output "with: ~s~%" (free-recall :scale 5 :decay 1.0)))

(actr-load "ACT-R:extras;base-level-inhibition;free-recall.lisp")
