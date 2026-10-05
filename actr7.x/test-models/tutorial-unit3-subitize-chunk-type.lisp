
(defun run-it () 
  (define-model f (sgp :v t))
  (with-open-file (f (translate-logical-pathname "ACT-R:tutorial;unit3;subitize-model.lisp") :direction :input)
    (loop
      (let ((form (read-line f)))
        (when (search "(chunk-type count " form :test 'string-equal)
          (model-output form)
          (return))))))

