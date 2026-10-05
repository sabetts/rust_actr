
(defun run-it () 
  (with-open-file (f (translate-logical-pathname "ACT-R:tutorial;unit3;sperling-model.lisp") :direction :input)
    (let ((started nil)(result "")(open 0))
      (loop
        (let ((form (read-line f)))
          (when (search ":seed" form :test 'string-equal)
            (model-output "; ~a" form)
            (return)))))))
          
  
(actr-load "ACT-R:tutorial;unit3;sperling-model.lisp")
