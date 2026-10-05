
(defun run-it () 
  (define-model f (sgp :v t))
  (with-open-file (f (translate-logical-pathname "ACT-R:test-models;models;broken-addition-v3.lisp") :direction :input)
    (loop
      (let ((form (read-line f)))
        (when (search "(test-goal" form :test 'string-equal)
          (aif (search "))" form :test 'string-equal)
              (model-output (subseq form 0 (1+ it)))
            (model-output form))
          (return))))))
