
(defun run-it () 
  (sgp :v nil)
  (run .05)
  (sgp :v t)
  (buffer-chunk goal))

(actr-load "ACT-R:test-models;models;broken-addition-v6.lisp")
