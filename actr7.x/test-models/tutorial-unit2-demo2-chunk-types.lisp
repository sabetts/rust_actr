
(defun run-it () 
  (define-model test (sgp :v t))
  (with-open-file (f (translate-logical-pathname "ACT-R:tutorial;unit2;demo2-model.lisp") :direction :input)
    (loop
      (let ((form (read f)))
        (when (eq (car form) 'define-model)
          (dolist (x form)
            (when (and (listp x) (eq (car x) 'chunk-type))
              (model-output "~(~s~)" x)))
          (return))))))
