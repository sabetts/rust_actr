
(defun run-it () 
  (define-model f (sgp :v t))
  (with-open-file (f (translate-logical-pathname "ACT-R:tutorial;unit3;perceptual-motor-issues-model.lisp") :direction :input)
    (loop
      (let ((form (read-line f nil :done)))
        (when (eq form :done) (return))
        
        (when (search "(chunk-type task" form :test 'string-equal)
          (model-output form)
          (return))
        ))))
          
