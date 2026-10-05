
(defun run-it () 
  (sgp :v nil)
  (run 1)
  (sgp :v t)
  (model-output "? (buffer-chunk goal retrieval)")
  (buffer-chunk goal retrieval))

(actr-load "ACT-R:test-models;models;broken-addition-v8.lisp")
