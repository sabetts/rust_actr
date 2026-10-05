

(defun run-it ()
  (sgp :v nil :cmdt nil)
  (let ((*standard-output* *out-file*))
    (display-results *siegler-data*)))


(load-act-r-code "ACT-R:tutorial;lisp;siegler.lisp")
