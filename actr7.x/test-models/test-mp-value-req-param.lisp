
;;; This model is a test for the mp-value request parameter of the
;;; declarative module.

(defun run-it ()
  (reset)
  (no-output (sgp :mp nil))
  (run 10)
  (reset)
  (run 30))

(load-act-r-model "ACT-R:test-models;models;test-mp-value-req-param.lisp")
