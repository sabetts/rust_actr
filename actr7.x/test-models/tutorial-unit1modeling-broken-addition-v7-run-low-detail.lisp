
(defun run-it () 
  (sgp :trace-detail low)
  (model-output "? (run 1)")
  (run 1))

(actr-load "ACT-R:test-models;models;broken-addition-v7.lisp")
