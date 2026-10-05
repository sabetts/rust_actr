
(defun run-it () 
  (define-model f (sgp :v t))
  (with-open-file (f (translate-logical-pathname "ACT-R:tutorial;unit5;declarative-issues.lisp") :direction :input)
    (let ((started nil)(open 0)(count 0))
      (loop
        (let ((form (read-line f)))
          (when (search "(set-base-levels" form :test 'string-equal)
            (setf started t))
          (when started
            (incf open (count #\( form))
            (decf open (count #\) form))
            
            (model-output form)
            
            (when (zerop open)
              (return))))))))
      
