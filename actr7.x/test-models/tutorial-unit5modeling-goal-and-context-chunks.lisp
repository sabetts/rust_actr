
(defun run-it () 
  (define-model f (sgp :v t))
  (with-open-file (f (translate-logical-pathname "ACT-R:tutorial;unit5;declarative-issues.lisp") :direction :input)
    (let ((started nil)(open 0)(count 0))
      (loop
        (let ((form (read-line f)))
          
          (when (or (search "(g1" form :test 'string-equal)
                    (search "(old-context" form :test 'string-equal))
            (setf started t))
          (when started
            (incf open (count #\( form))
            (decf open (count #\) form))
            
            
            (cond ((zerop open)
                   (incf count)
                   (model-output form))
                  ((< open 0)
                   (incf count)
                   (model-output (subseq form 0 (1- (length form)))))
                  (t
                   (model-output form)))
            (when (= count 2)
              (return))))))))
      
