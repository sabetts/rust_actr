
(defun run-it () 
  (define-model f (sgp :v t))
  (with-open-file (f (translate-logical-pathname "ACT-R:tutorial;unit1;count.lisp") :direction :input)
    (let ((started nil)(open 0)(count 0))
      (loop
        (let ((form (read-line f)))
          (when (search "(p start" form :test 'string-equal)
            (setf started t))
          (when started
            (incf open (count #\( form))
            (decf open (count #\) form))
            
            (if (minusp open)
                (progn
                  (model-output (subseq form 0 (+ (length form) open)))
                  (setf open 0))
              (model-output form))

            (when (zerop open)
              (return))))))))
