
(clear-all)
(defun run-it () (run-test nil))
(ssp :starting-parameters (:style-warnings nil :testing-function set-testing-traces))
(actr-load "ACT-R:examples;rotational-vision-test-2.lisp")
