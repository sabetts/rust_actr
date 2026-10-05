
(defun run-it () 
  (define-model f (sgp :v t))
  (with-open-file (f (translate-logical-pathname "ACT-R:tutorial;unit5;1hit-blackjack-model.lisp") :direction :input)
    (let ((started nil)(open 0)(count 0))
      (loop
        (let ((form (read-line f)))
          (when (search "(define-chunks" form :test 'string-equal)
            (model-output form)
            (return)))))))
      
