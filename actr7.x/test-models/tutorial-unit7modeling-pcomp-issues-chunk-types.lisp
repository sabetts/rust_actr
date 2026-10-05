(defun run-it () 
  (define-model f (sgp :v t))
  (with-open-file (f (translate-logical-pathname "ACT-R:tutorial;unit7;production-compilation-issues-model.lisp") :direction :input)
    (let ((started nil)(open 0)(count 0))
      (loop
        (let ((form (read-line f nil :done)))
          (when (eq form :done) (return))
          (when (search "(chunk-type" form :test 'string-equal)
            (model-output form)))))))