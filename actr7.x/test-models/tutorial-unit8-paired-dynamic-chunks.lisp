
(defun run-it () 
  (define-model f (sgp :v t))
  (with-open-file (f (translate-logical-pathname "ACT-R:tutorial;unit8;paired-dynamic-model.lisp") :direction :input)
    (let ((started nil)(open 0)(count 0)(data nil) (which nil)(over nil) (under nil))
      (loop
        (let ((form (read-line f nil :done)))
          (when (eq form :done) (return))
          (when (search "(op" form :test 'string-equal)
            (if (search "))" form :test 'string-equal)
                (model-output (subseq form 0 (1- (length form))))
              (model-output form))))))))
