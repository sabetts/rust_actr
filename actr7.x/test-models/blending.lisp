
(defun run-it () 
  (run 5)
  (clear-all)
  (undefine-module blending)
  (setf *modules* (remove "blending" *modules* :test 'string-equal)))

(actr-load "ACT-R:extras;blending;blending-test-7.lisp")
