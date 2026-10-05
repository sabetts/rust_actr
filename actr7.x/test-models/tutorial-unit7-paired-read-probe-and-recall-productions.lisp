
(defun run-it () 
  (define-model f (sgp :v t))
  (with-open-file (f (translate-logical-pathname "ACT-R:tutorial;unit4;paired-model.lisp") :direction :input)
    (let ((started nil)(open 0)(count 0)(data nil) (which nil)(read-probe nil) (recall nil))
      (loop
        (let ((form (read-line f nil :done)))
          (when (eq form :done) (return))
          (when (or (search "(p read-probe" form :test 'string-equal)
                    (search "(p recall" form :test 'string-equal))
            (setf started t)
            (setf which (search "(p read-probe" form :test 'string-equal))
            (setf data nil))
          
          (when started
            (incf open (count #\( form))
            (decf open (count #\) form))
            
            (push-last form data)
            
            (when (zerop open)
              (setf started nil)
              (if which
                  (setf read-probe data)
                (setf recall data))))))
      
      (cond ((< (length read-probe) (length recall))
             (setf read-probe (append read-probe (make-list (- (length recall) (length read-probe)) :initial-element " "))))
            ((> (length read-probe) (length recall))
             (setf recall (append recall (make-list (- (length read-probe) (length recall)) :initial-element " ")))))
      (let ((m (max (apply 'max (mapcar 'length read-probe))
                    (apply 'max (mapcar 'length recall)))))
        (format *out-file* "~{~va     ~a~%~}" (mapcan (lambda (x y) (list m x y)) read-probe recall))))

      ))
      
