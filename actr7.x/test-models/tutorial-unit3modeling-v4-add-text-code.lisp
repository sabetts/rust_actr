
(defun run-it () 
  (define-model f (sgp :v t))
  (model-output "Lisp:~%")
  (with-open-file (f (translate-logical-pathname "ACT-R:tutorial;lisp;pm-issues.lisp") :direction :input)
    (let ((started nil)(result "")(open 0))
      (loop
        (let ((form (read-line f nil :done)))
          (when (eq form :done) (return))
          (when (search "(add-text-to-exp-window" form :test 'string-equal)
            (model-output form))))))
  
  (model-output "~%Python:~%")
  ;ACL doesn't like _ in logicals...
  
  (with-open-file (f (merge-pathnames "pm_issues.py" (translate-logical-pathname "ACT-R:tutorial;python;")) :direction :input)
    (let ((started nil)(result "")(open 0))
      (loop
        (let ((form (read-line f nil :done)))
          (when (eq form :done) (return))
          (when (search "add_text_to_exp_window" form :test 'string-equal)
            (model-output form)))))))
  
