
(defun run-it () 
  (define-model test (sgp :v t))
  (with-open-file (f (translate-logical-pathname "ACT-R:tutorial;unit1;count.lisp") :direction :input)
    (loop
      (let ((form (read f)))
        (when (eq (car form) 'define-model)
          (dolist (x form)
            (when (and (listp x) (eq (car x) 'add-dm))
              (model-output "(add-dm")
              (model-output "~{ (~{~a~^ ~})~^~%~})" (mapcar (lambda (z)
                                                              (mapcar (lambda (q)
                                                                        (if (eq q 'isa)
                                                                            "ISA"
                                                                          (string-downcase (symbol-name q))))
                                                                z))
                                                      (cdr x)))
              (return-from run-it))))))))
