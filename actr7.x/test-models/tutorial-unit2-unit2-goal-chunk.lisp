
(defun run-it () 
  (define-model f (sgp :v t))
  (with-open-file (f (translate-logical-pathname "ACT-R:tutorial;unit2;unit2-assignment-model.lisp") :direction :input)
    (loop
      (let ((form (read-line f nil :done)))
        (when (search "goal-focus" form :test 'string-equal)
          (model-output form)
          (return))))))
                  
  
