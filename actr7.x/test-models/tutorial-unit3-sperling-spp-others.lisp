
(defun run-it () 
  (with-open-file (f (translate-logical-pathname "ACT-R:tutorial;unit3;sperling-model.lisp") :direction :input)
    (let ((started nil)(result "")(open 0))
      (loop
        (let ((form (read-line f nil :done)))
          (when (eq form :done) (return))
          (when (and (search "(spp" form :test 'string-equal)
                     (not (search "start-report" form :test 'string-equal)))
            (model-output form)))))))
          
  
(actr-load "ACT-R:tutorial;unit3;sperling-model.lisp")
