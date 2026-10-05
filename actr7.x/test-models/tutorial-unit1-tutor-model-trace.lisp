
(defun run-it () 
  ;; Want to decrease the spacing
  
  (let ((f (fdefinition 'max-module-name-length)))
    (setf (fdefinition 'max-module-name-length) (lambda () 11))
    
    (run 10)
    (setf (fdefinition 'max-module-name-length) f)))


(load-act-r-model "ACT-R:tutorial;unit1;tutor-model-solution.lisp")
