
(defun run-it () 
  (model-output "? (whynot direct-verify fail)")
  (multiple-value-bind (a) (whynot direct-verify fail)
    (model-output "~s" a))
  )
    
(load-act-r-model "ACT-R:tutorial;unit1;semantic.lisp")
