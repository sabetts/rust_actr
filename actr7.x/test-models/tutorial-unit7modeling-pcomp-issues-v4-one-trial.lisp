
(actr-load "ACT-R:tutorial;lisp;pcomp-issues.lisp")
(actr-load "ACT-R:test-models;models;production-compilation-issues-model-v4.lisp")

(defun run-it () 
  (pcomp-issues-trials 1 t nil))