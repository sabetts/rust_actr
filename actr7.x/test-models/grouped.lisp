
(ssp :starting-parameters (:testing-function grouped-test))

(defun grouped-test ()
  (sgp :act t)
  (set-testing-traces))

(defun run-it ()
  (grouped-recall))

(actr-load "ACT-R:tutorial;lisp;grouped.lisp")
