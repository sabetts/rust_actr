
(ssp :starting-parameters (:seed (57531033409 0) :model-warnings nil :testing-function set-testing-traces))

(defun run-it () 
  (run-test 40)
  (clear-all)
  (undefine-module :tracker)
  (setf *modules* (remove "tracker" *modules* :test 'string-equal)))


(actr-load "ACT-R:extras;tracker;simple.lisp")
