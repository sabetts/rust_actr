
(defun run-it () 
  (with-open-file (f (translate-logical-pathname "ACT-R:tutorial;unit2;demo2-model.lisp") :direction :input)
    (loop
      (let ((form (read-line f nil :done)))
        (when (eq form :done)
          (return))
        (when (search "(sgp" form :test 'string-equal)
          (model-output form))))))
                  
  
(actr-load "ACT-R:tutorial;unit2;demo2-model.lisp")
