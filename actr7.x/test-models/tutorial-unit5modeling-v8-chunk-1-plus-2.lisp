
(ssp :starting-parameters (:model-warnings nil :testing-function set-testing-traces))

(actr-load "ACT-R:test-models;models;declarative-issues-v8.lisp")


(defun run-it ()
  (pprint-chunks 1+2=3))