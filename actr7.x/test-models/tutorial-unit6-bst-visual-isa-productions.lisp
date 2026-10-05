
(defun run-it () 
  (define-model f (sgp :v t))
  (with-open-file (f (translate-logical-pathname "ACT-R:tutorial;unit6;bst-model.lisp") :direction :input)
    (let ((started nil)(open 0)(count 0))
      (loop
        (let ((form (read-line f nil :done)))
          (when (eq form :done) (return))
          (when (or (search "(p encode-line-goal" form :test 'string-equal)
                    (search "(p read-done" form :test 'string-equal))
            (setf started t))
          (when started
            (incf open (count #\( form))
            (decf open (count #\) form))
            
            (model-output form)
            
            (when (zerop open)
              (setf started nil)
              (model-output ""))))))))
      
