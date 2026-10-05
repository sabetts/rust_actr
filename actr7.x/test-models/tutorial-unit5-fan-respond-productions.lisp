
(defun run-it () 
  (define-model f (sgp :v t))
  (with-open-file (f (translate-logical-pathname "ACT-R:tutorial;unit5;fan-model.lisp") :direction :input)
    (let ((started nil)(open 0)(count 0))
      (loop
        (let ((form (read-line f)))
          (when (or (search "(p yes" form :test 'string-equal)
                    (search "(p mismatch-" form :test 'string-equal))
            (setf started t)
            )
          (when started
            (incf open (count #\( form))
            (decf open (count #\) form))
            
            (if (minusp open)
                (progn
                  (model-output (subseq form 0 (+ (length form) open)))
                  (setf open 0))
              (model-output form))

            (when (zerop open)
              (incf count)
              (setf started nil)
              (if (= count 3)
                  (return)
                (model-output "")))))))))
