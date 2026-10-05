
(defun run-it () 
  (define-model f (sgp :v t))
  (with-open-file (f (translate-logical-pathname "ACT-R:test-models;models;declarative-issues-v3.lisp") :direction :input)
    (let ((started nil)(open 0)(count 0))
      (loop
        (let ((form (read-line f)))
          (when (search "(sgp :esc" form :test 'string-equal)
            (model-output form)
              (return)))))))
      
