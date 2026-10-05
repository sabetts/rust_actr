
(defun run-it () 
  (sgp :v nil)
  (run 1)
  (sgp :v t)
  (model-output "? (buffer-chunk goal)")
  (buffer-chunk goal))

(actr-load "ACT-R:test-models;models;broken-addition-v9.lisp")
