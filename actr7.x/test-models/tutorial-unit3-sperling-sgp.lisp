
(defun run-it () 
  (with-open-file (f (translate-logical-pathname "ACT-R:tutorial;unit3;sperling-model.lisp") :direction :input)
    (let ((started nil)(result "")(open 0))
      (loop
        (let ((form (read-line f)))
          (when (search ":declarative-finst-span" form :test 'string-equal)
            (model-output form)
            (return)))))))
          
  
(actr-load "ACT-R:tutorial;unit3;sperling-model.lisp")
