
(defun run-it () 
  (define-model f (sgp :v t))
  (with-open-file (f (translate-logical-pathname "ACT-R:test-models;models;production-compilation-issues-model-v5.lisp") :direction :input)
    (let ((started nil)(open 0)(count 0))
      (loop
        (let ((form (read-line f nil :done)))
          (when (eq form :done) (return))
          (when (search "(p encode-num-" form :test 'string-equal)
            (setf started t))
          (when started
            (incf open (count #\( form))
            (decf open (count #\) form))
            
            (model-output form)
            
            (when (zerop open)
              (setf started nil))))))))
      
