
(actr-load "ACT-R:tutorial;lisp;pcomp-issues.lisp")

(defun run-it () 
  (reset)
  (sgp :seed (100 1) :v nil)
  (pcomp-issues-trials 1 nil nil)
  (sgp :v t)
  (buffer-chunk imaginal))