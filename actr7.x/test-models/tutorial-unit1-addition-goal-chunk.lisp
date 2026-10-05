
(defun run-it () 
  (define-model test (sgp :v t))
  (with-open-file (f (translate-logical-pathname "ACT-R:tutorial;unit1;addition.lisp") :direction :input)
    (loop
      (let ((form (read-line f)))
        (when (search "goal-focus" form :test 'string-equal)
          (model-output form)
          (return))))))
        
