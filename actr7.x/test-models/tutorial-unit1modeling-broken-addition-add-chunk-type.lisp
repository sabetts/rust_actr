
(defun run-it () 
  (define-model test (sgp :v t))
  (with-open-file (f (translate-logical-pathname "ACT-R:tutorial;unit1;broken-addition.lisp") :direction :input)
    (loop
      (let ((form (read-line f)))
        (when (search "(chunk-type add" form :test 'string-equal)
          (model-output form)
          (return))))))
