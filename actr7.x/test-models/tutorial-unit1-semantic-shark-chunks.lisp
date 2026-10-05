
(defun run-it () 
  (define-model f (sgp :v t))
  (with-open-file (f (translate-logical-pathname "ACT-R:tutorial;unit1;semantic.lisp") :direction :input)
    (let ((count 0))
      (loop
        (let ((form (read-line f)))
          (when (search " shark " form :test 'string-equal)
            (model-output form)
            (incf count)
            (when (= count 3)
              (return))))))))
