(defun run-it () 
  ;; Want to decrease the spacing
  
  (let ((f (fdefinition 'max-module-name-length)))
    (setf (fdefinition 'max-module-name-length) (lambda () 13))
    
    (run 10)
    (setf (fdefinition 'max-module-name-length) f)))

(actr-load "ACT-R:tutorial;unit1;count.lisp")