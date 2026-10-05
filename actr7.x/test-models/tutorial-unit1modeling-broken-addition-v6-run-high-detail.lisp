
(defun run-it () 
  (sgp :trace-detail high)
  (model-output "? (run 10)")
  (run 10))

(actr-load "ACT-R:test-models;models;broken-addition-v6.lisp")
