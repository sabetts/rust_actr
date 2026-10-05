
(defun run-it () 
  (with-open-file (f (translate-logical-pathname "ACT-R:tutorial;unit1;semantic.lisp") :direction :input)
    (let ((started nil)(result "")(open 0))
      (loop
        (let ((form (read-line f)))
          (when (search "(p fail" form :test 'string-equal)
            (setf started t))
          (when started
            (setf result (concatenate 'string result form))
            (incf open (count #\( form))
            (decf open (count #\) form))
            (if (zerop open)
                (progn
                  (model-output result)
                  (return))
              (setf result (concatenate 'string result (format nil "~%"))))))))))
          
  
(actr-load "ACT-R:tutorial;unit1;semantic.lisp")
