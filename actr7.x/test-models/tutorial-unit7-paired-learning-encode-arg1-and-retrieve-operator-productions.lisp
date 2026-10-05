
(defun run-it () 
  (define-model f (sgp :v t))
  (with-open-file (f (translate-logical-pathname "ACT-R:tutorial;unit7;paired-learning-model.lisp") :direction :input)
    (let ((started nil)(open 0)(count 0)(data nil) (which nil)(encode nil) (retrieve nil))
      (loop
        (let ((form (read-line f nil :done)))
          (when (eq form :done) (return))
          (when (or (search "(p encode-arg1" form :test 'string-equal)
                    (search "(p retrieve-operator" form :test 'string-equal))
            (setf started t)
            (setf which (search "(p encode-arg1" form :test 'string-equal))
            (setf data nil))
          
          (when started
            (incf open (count #\( form))
            (decf open (count #\) form))
            
            (push-last form data)
            
            (when (zerop open)
              (setf started nil)
              (if which
                  (setf encode data)
                (setf retrieve data))))))
      
      (cond ((< (length encode) (length retrieve))
             (setf encode (append encode (make-list (- (length retrieve) (length encode)) :initial-element " "))))
            ((> (length encode) (length retrieve))
             (setf retrieve (append retrieve (make-list (- (length encode) (length retrieve)) :initial-element " ")))))
      (let ((m (max (apply 'max (mapcar 'length encode))
                    (apply 'max (mapcar 'length retrieve)))))
        (format *out-file* "~{~va     ~a~%~}" (mapcan (lambda (x y) (list m x y)) encode retrieve))))

      ))
      
