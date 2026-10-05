

(defun run-it ()
  (sgp :v nil :cmdt nil :seed (1 0))
  (let ((x (onehit-hands 5)))
    (sgp-fct (list :v *out-file*))
    (model-output "? (onehit-hands 5)~%~s" x)))
    

(load-act-r-code "ACT-R:tutorial;lisp;onehit.lisp")
