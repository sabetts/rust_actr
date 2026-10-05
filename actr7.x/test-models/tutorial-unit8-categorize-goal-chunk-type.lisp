
(defun run-it () 
  (define-model f (sgp :v t))
  (with-open-file (f (translate-logical-pathname "ACT-R:tutorial;unit8;categorize-model.lisp") :direction :input)
    (let ((count 0))
      (loop
        (let ((form (read-line f)))
          (when (search "chunk-type goal" form :test 'string-equal)
            (if (search "))" form :test 'string-equal)
                (model-output (subseq form 0 (1- (length form))))
              (model-output form))
              (return)))))))
