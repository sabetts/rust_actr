
(defun run-it () 
  (define-model f (sgp :v t))
  (with-open-file (f (translate-logical-pathname "ACT-R:test-models;models;broken-addition-v11.lisp") :direction :input)
    (loop
      (let ((form (read-line f)))
        (when (search "(six isa" form :test 'string-equal)
          (model-output form)
          (return))))))          

