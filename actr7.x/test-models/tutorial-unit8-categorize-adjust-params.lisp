
(defun run-it () 
  (define-model f (sgp :v t))
  (with-open-file (f (translate-logical-pathname "ACT-R:tutorial;unit8;categorize-model.lisp") :direction :input)
    (let ((count 0))
      (loop
        (let ((form (read-line f)))
          (when (search "(sgp :mp" form :test 'string-equal)
            (model-output form)
            (return)))))))
