
(defun run-it () 
  (sgp :trace-detail low)
  (model-output "? (run 10)")
  (run 10))

(actr-load "ACT-R:test-models;models;broken-addition-v12.lisp")
