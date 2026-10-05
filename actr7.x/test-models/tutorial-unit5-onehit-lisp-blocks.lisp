

(defun run-it ()
  (sgp :v nil :cmdt nil :seed (1 0))
  (let ((x (onehit-blocks 2 5)))
    (sgp-fct (list :v *out-file*))
    (model-output "? (onehit-blocks 2 5)~%~s" x)))
    

(load-act-r-code "ACT-R:tutorial;lisp;onehit.lisp")
