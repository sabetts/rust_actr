;;; This test model has a bunch of bad production definitions
;;; which should be rejected with appropriate warnings.



(defun run-it ()
  (reset)
  (run 10))

(load-act-r-model "ACT-R:test-models;models;test-bad-productions.lisp")
