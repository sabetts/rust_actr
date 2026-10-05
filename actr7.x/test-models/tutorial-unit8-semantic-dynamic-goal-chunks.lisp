
(defun run-it () 
  (define-model f (sgp :v t))
  (with-open-file (f (translate-logical-pathname "ACT-R:tutorial;unit8;semantic-dynamic-model.lisp") :direction :input)
    (let ((count 0))
      (model-output "(define-chunks")
      (loop
        (let ((form (read-line f)))
          (when (search "isa test-attribute" form :test 'string-equal)
            (model-output form)
            (when (search "))" form :test 'string-equal)
              (return))))))))
