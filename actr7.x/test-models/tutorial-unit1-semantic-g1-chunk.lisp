
(defun run-it () 
  (define-model f (sgp :v t))
  (with-open-file (f (translate-logical-pathname "ACT-R:tutorial;unit1;semantic.lisp") :direction :input)
    (loop
      (let ((form (read-line f)))
        (when (search "category bird))" form :test 'string-equal)
          (model-output form)
          (return))))))
