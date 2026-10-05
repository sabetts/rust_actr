
(defun run-it () 
  (define-model f (sgp :v t))
  (with-open-file (f (translate-logical-pathname "ACT-R:tutorial;unit8;simple-ppm-model.lisp") :direction :input)
    (let ((started nil)(open 0)(count 0)(data nil) (which nil)(small nil) (medium nil)(large nil))
      (loop
        (let ((form (read-line f nil :done)))
          (when (eq form :done) (return))
          (when (or (search "(p small" form :test 'string-equal)
                    (search "(p medium" form :test 'string-equal)
                    (search "(p large" form :test 'string-equal))
            (setf started t)
            (setf which (cond ((search "(p small" form :test 'string-equal) 1)
                              ((search "(p medium" form :test 'string-equal) 2)
                              ((search "(p large" form :test 'string-equal) 3)))
            (setf data nil))
          
          (when started
            (incf open (count #\( form))
            (decf open (count #\) form))
            
            
            (if (< open 0)
                (push-last (subseq form 0 (+ (length form) open)) data)
              (push-last form data))
            
            (when (<= open 0)
              (setf started nil)
              (case which
                (1 (setf small data))
                (2 (setf medium data))
                (3 (setf large data)))))))
      
      (let ((m (max (apply 'max (mapcar 'length small))
                    (apply 'max (mapcar 'length medium))
                    (apply 'max (mapcar 'length large)))))
        
        (format *out-file* "~{~va      ~va      ~a~%~}" 
          (mapcan (lambda (x y z) (list m x m y z)) small medium large))))))
      
