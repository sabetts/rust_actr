
(defun run-it () 
  (model-output "? (run 1)")
  (multiple-value-bind (a b c) (run 1)
    (model-output "~s" a)
    (model-output "~s" b)
    (model-output "~s" c)))
    
(load-act-r-model "ACT-R:tutorial;unit1;semantic.lisp")
