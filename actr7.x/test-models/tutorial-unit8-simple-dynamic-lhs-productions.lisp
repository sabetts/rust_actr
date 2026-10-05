
(defun run-it () 
  (define-model f (sgp :v t))
  (with-open-file (f (translate-logical-pathname "ACT-R:tutorial;unit8;simple-dynamic-model.lisp") :direction :input)
    (let ((started nil)(open 0)(count 0)(data nil) (which nil)(start nil) (retrieve nil))
      (loop
        (let ((form (read-line f nil :done)))
          (when (eq form :done) (return))
          (when (or (search "(p start" form :test 'string-equal)
                    (search "(p retrieve-first" form :test 'string-equal))

            (setf started t)
            (setf which (search "start" form :test 'string-equal))
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
                  (setf start data)
                (setf retrieve data))))))
      (let ((ls (length start))
            (lr (length retrieve)))
        (cond ((< ls lr)
               (dotimes (i (- lr ls)) (push-last "" start)))
              ((< lr ls)
               (dotimes (i (- ls lr)) (push-last "" retrieve)))))
              

      
      (let ((m (max (apply 'max (mapcar 'length start))
                    (apply 'max (mapcar 'length retrieve)))))
        
        (format *out-file* "~{~va        ~a~%~}" 
          (mapcan (lambda (x y) (list m x y)) start retrieve))))))
      
