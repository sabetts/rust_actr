
(defun run-it () 
  (define-model test (sgp :v t))
  (with-open-file (f (translate-logical-pathname "ACT-R:tutorial;unit1;addition.lisp") :direction :input)
    (loop
      (let ((form (read f)))
        (when (eq (car form) 'define-model)
          (dolist (x form)
            (when (and (listp x) (eq (car x) 'chunk-type)
                       (eq (second x) 'add))
              (model-output "~(~s~)" x)))
          (return))))))
