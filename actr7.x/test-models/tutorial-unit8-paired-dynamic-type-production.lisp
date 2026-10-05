
(defun run-it () 
  (define-model f (sgp :v t))
  (with-open-file (f (translate-logical-pathname "ACT-R:tutorial;unit8;paired-dynamic-model.lisp") :direction :input)
    (let ((started nil)(open 0)(count 0)(data nil) (which nil)(over nil) (under nil))
      (loop
        (let ((form (read-line f nil :done)))
          (when (eq form :done) (return))
          (when (search "(p type" form :test 'string-equal)
            (setf started t))
          
          (when started
            (incf open (count #\( form))
            (decf open (count #\) form))
            
            (if (<= open 0)
                (progn
                  (if (zerop open)
                      (model-output form)
                    (model-output (subseq form 0 (+ (length form) open))))
                  (return))
              (model-output form))))))))
              
