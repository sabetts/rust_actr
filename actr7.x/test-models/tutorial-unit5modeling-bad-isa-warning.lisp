
(ssp :starting-parameters (:model-warnings nil :testing-function set-testing-traces))

(actr-load "ACT-R:tutorial;unit5;declarative-issues.lisp")

(defun run-it ()
  (delete-production 'p1)
  (sgp :model-warnings t)
  (p p1 ==> +retrieval> isa simple-value arg1 nil))