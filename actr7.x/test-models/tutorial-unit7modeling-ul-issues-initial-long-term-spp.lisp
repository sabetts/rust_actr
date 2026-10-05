
(actr-load "ACT-R:tutorial;lisp;ul-issues.lisp")

(defun run-it () 
  (sgp :v nil)
  (run 700)
  (sgp :v t)
  (spp choose-a choose-b))