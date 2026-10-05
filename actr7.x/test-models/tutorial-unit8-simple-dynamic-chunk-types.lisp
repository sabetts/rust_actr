
(defun run-it () 
  (define-model f (sgp :v t))
  (with-open-file (f (translate-logical-pathname "ACT-R:tutorial;unit8;simple-dynamic-model.lisp") :direction :input)
    (let ((started nil)(open 0)(count 0)(data nil) (which nil)(start nil) (retrieve nil))
      (loop
        (let ((form (read-line f nil :done)))
          (when (eq form :done) (return))
          (when (search "(chunk-type" form :test 'string-equal)
            (model-output form)))))))
