(defun run-it () 
  (define-model f (sgp :v t))
  (with-open-file (f (translate-logical-pathname "ACT-R:test-models;models;declarative-issues-v1.lisp") :direction :input)
    (let ((started nil)(open 0)(count 0))
      (loop
        (let ((form (read-line f nil :done)))
          (when (eq form :done) (return))
          (when (search "(chunk-type simple-value" form :test 'string-equal)
            (model-output form)
            (model-output ""))
          
          (when  (search "isa simple-value result" form :test 'string-equal)
            (model-output form)))))))