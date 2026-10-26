
(defun run-it () 
  (reset)
  (sgp :cia nil)
  (run 1)
  (reset)
  (sgp :cia t)
  (run 1)
  (reset)
  (sgp :cia t :rir t)
  (run 1))

(actr-load "ACT-R:test-models;models;retain-indirect-retrievals.lisp")
