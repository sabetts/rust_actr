
(defun run-it () 
  (define-model f (sgp :v t))
  (with-open-file (f (translate-logical-pathname "ACT-R:tutorial;lisp;sperling.lisp") :direction :input)
    (loop
      (let ((form (read-line f nil :done)))
        (when (search "(run " form :test 'string-equal)
          (model-output form)
          (return)))))
  (model-output "")
  (with-open-file (f (translate-logical-pathname "ACT-R:tutorial;python;sperling.py") :direction :input)
    (loop
      (let ((form (read-line f nil :done)))
        (when (search "actr.run(" form :test 'string-equal)
          (model-output form)
          (return))))))
                  
  
