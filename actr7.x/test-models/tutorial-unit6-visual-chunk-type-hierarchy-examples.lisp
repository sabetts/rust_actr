
(defun run-it () 
  (define-model f (sgp :v t))
  (with-open-file (f (translate-logical-pathname "ACT-R:core-modules;vision.lisp") :direction :input)
    (let ((started nil)(open 0)(count 0))
      (loop
        (let ((form (read-line f nil :done)))
          (when (eq form :done) (return))
          (when (or (search "(chunk-type visual-object" form :test 'string-equal)
                    (search "(chunk-type (text" form :test 'string-equal)
                    (search "(chunk-type (line " form :test 'string-equal))
            (model-output form)))))))
