
(defun run-it () 
  (define-model f (sgp :v t))
  (with-open-file (f (translate-logical-pathname "ACT-R:tutorial;unit6;bst-model.lisp") :direction :input)
    (let ((started nil)(open 0)(count 0))
      (loop
        (let ((form (read-line f nil :done)))
          (when (eq form :done) (return))
          
          (when (search ":reward" form :test 'string-equal)
            (setf started t))
          (when started
            (incf open (count #\( form))
            (decf open (count #\) form))
            
            (cond ((zerop open)
                   (model-output form)
                   (setf started nil))
                  ((< open 0)
                   (model-output (subseq form 0 (1- (length form))))
                   (setf started nil))
                  (t
                   (model-output form)))))))))      
