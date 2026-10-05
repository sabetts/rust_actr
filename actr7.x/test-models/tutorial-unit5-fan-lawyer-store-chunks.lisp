
(defun run-it () 
  (define-model test (sgp :v t))
  (with-open-file (f (translate-logical-pathname "ACT-R:tutorial;unit5;fan-model.lisp") :direction :input)
    (let ((started nil))
      (loop
        (let ((form (read-line f nil :done)))
          (when (eq form :done) (return))
          (when (or (search "\"lawyer\"" form :test 'string-equal)
                    (search "\"store\"" form :test 'string-equal))
            (model-output form)))))))
