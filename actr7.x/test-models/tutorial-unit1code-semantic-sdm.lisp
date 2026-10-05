
(defun run-it () 
  
  (model-output "? (sdm object shark)")
  (multiple-value-bind (a) (sdm object shark)
    (model-output "~s" a))
  (model-output "~%? (sdm - attribute category - attribute nil)")
  (multiple-value-bind (a) (sdm - attribute category - attribute nil)
    (model-output "~s" a))
  )
    
(load-act-r-model "ACT-R:tutorial;unit1;semantic.lisp")
