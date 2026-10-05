
(defun run-it () 
  (sgp :v nil)
  (run 1)
  (sgp :v t)
  (model-output "? (whynot increment-count)")
  (whynot increment-count))

(actr-load "ACT-R:test-models;models;broken-addition-v8.lisp")
