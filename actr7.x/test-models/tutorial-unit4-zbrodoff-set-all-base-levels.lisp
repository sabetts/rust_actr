(defun run-it () 
  (define-model f (sgp :v t))
  (with-open-file (f (translate-logical-pathname "ACT-R:tutorial;unit4;zbrodoff-model.lisp") :direction :input)
    (let ((started nil)(result "")(open 0))
      (loop
        (let ((form (read-line f)))
          (when (search "(set-all-base-levels" form :test 'string-equal)
            (model-output form)
            (return)))))))