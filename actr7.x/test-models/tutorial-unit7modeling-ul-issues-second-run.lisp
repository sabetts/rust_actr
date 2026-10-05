
(actr-load "ACT-R:tutorial;lisp;ul-issues.lisp")


(defun run-it () 
  (with-parameters (:v nil)
    (run 6.95))
  (run 14))