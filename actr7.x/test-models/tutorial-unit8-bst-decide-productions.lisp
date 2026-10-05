
(defun run-it () 
  (define-model f (sgp :v t))
  (with-open-file (f (translate-logical-pathname "ACT-R:tutorial;unit8;bst-ppm-model.lisp") :direction :input)
    (let ((started nil)(open 0)(count 0)(data nil) (which nil)(over nil) (under nil))
      (loop
        (let ((form (read-line f nil :done)))
          (when (eq form :done) (return))
          (when (search "(p decide-" form :test 'string-equal)
            (setf started t)
            (setf which (search "over" form :test 'string-equal))
            (setf data nil))
          
          (when started
            (incf open (count #\( form))
            (decf open (count #\) form))
            
            
            (if (< open 0)
                (push-last (subseq form 0 (+ (length form) open)) data)
              (push-last form data))
            
            (when (<= open 0)
              (setf started nil)
              (if which
                  (setf over data)
                (setf under data))))))
      
      (let ((m (max (apply 'max (mapcar 'length over))
                    (apply 'max (mapcar 'length under)))))
        
        (format *out-file* "~{~va        ~a~%~}" 
          (mapcan (lambda (x y) (list m x y)) over under))))))
      
