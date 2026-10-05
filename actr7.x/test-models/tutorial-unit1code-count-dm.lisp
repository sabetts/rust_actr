
(defun run-it () 
  
  (model-output "? (dm)")
  (multiple-value-bind (a) (dm)
    (model-output "~s" a))
  (model-output "~%? (dm four one)")
  (multiple-value-bind (a) (dm four one)
    (model-output "~s" a))
  )
    
(load-act-r-model "ACT-R:tutorial;unit1;count.lisp")
