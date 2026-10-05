
(defun run-it () 
  (define-model test (sgp :v t))
  (with-open-file (f (translate-logical-pathname "ACT-R:tutorial;unit5;fan-model.lisp") :direction :input)
    (let ((started nil))
      (loop
        (let ((form (read-line f nil :done)))
          
          (if (or (search "(add-dm" form :test 'string-equal)
                  (search "isa comprehend-" form :test 'string-equal))
              (progn
                (setf started t)
                (model-output form))
            (when started
              (model-output "   ...)")
              (return))))))))
