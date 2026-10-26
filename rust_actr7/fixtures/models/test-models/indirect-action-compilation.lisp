
(defun run-it () 
  (reset)
  (sgp :cia nil)
  (run .2)
  (reset)
  (sgp :cia t)
  (run .2))

(actr-load "ACT-R:test-models;models;indirect-action-compilation.lisp")
