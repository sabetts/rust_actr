
(defun run-it () 
  (model-output "? (sdm - attribute category)~%...")
  (multiple-value-bind (a) (no-output (sdm - attribute category))
    (model-output "~s" a))
  )
    
(load-act-r-model "ACT-R:tutorial;unit1;semantic.lisp")
