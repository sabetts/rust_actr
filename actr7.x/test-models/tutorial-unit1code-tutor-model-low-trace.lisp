

(defun run-it ()
  (sgp :trace-detail low)
  (let ((f (fdefinition 'max-module-name-length)))
    (setf (fdefinition 'max-module-name-length) (lambda () 11))
    
    (run 10)
    (setf (fdefinition 'max-module-name-length) f))
  )

(load-act-r-model "ACT-R:tutorial;unit1;tutor-model-solution.lisp")
