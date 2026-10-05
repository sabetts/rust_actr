
(defun run-it () 
  (sgp :v nil)
  (run 1)
  (sgp :v t)
  
  (model-output "? (buffer-chunk)")
  (multiple-value-bind (a) (buffer-chunk)
    (model-output "~s" a)))
    
(load-act-r-model "ACT-R:tutorial;unit1;semantic.lisp")
