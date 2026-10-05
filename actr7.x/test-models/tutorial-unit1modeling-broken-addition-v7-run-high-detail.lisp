
(defun run-it () 
  (sgp :trace-detail high)
  (model-output "? (run .3)")
  (run .3))

(actr-load "ACT-R:test-models;models;broken-addition-v7.lisp")
