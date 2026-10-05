
(defun run-it () 
  
  (model-output "? (buffer-status goal visual)")
  (multiple-value-bind (a) (buffer-status goal visual)
    (model-output "~s" a)))
    
(load-act-r-model "ACT-R:tutorial;unit1;addition.lisp")
