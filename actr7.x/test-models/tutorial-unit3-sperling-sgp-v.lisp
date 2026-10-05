
(defun run-it () 
  (with-open-file (f (translate-logical-pathname "ACT-R:tutorial;unit3;sperling-model.lisp") :direction :input)
    (let ((started nil)(result "")(open 0))
      (loop
        (let ((form (read-line f)))
          (awhen (search ":v " form :test 'string-equal)
            (model-output "~a nil ~a" (subseq form 0 (+ it 2)) (subseq form (+ it 5)))
            (return)))))))
          
  
(actr-load "ACT-R:tutorial;unit3;sperling-model.lisp")
