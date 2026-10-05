
(ssp :starting-parameters (:use-tree t))

(defun run-it () (run 10)
  (print-conflict-tree))

(actr-load "ACT-R:tutorial;unit1;count.lisp")
