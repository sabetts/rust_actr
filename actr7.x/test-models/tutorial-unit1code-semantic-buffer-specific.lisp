
(defun run-it () 
  (sgp :v nil)
  (run 1)
  (sgp :v t)
  
  (model-output "? (buffer-chunk retrieval goal)")
  (multiple-value-bind (a) (buffer-chunk retrieval goal)
    (model-output "~s" a)))
    
(load-act-r-model "ACT-R:tutorial;unit1;semantic.lisp")
