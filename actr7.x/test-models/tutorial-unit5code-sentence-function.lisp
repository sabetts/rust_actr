
(defun run-it () 
  (define-model f (sgp :v t))
  (with-open-file (f (merge-pathnames "fan_no_pm.py" (translate-logical-pathname "ACT-R:tutorial;python;")) :direction :input)
    (let ((started nil)(open 0)(count 0))
      (loop
        (let ((form (read-line f)))
          (when (search "def sentence" form :test 'string-equal)
            (setf started t))
                      
          (when (search "do_person_location" form :test 'string-equal)
            (return))

          
          (when started            
            (model-output form)))))))
      
