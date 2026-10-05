
(ssp :starting-parameters (:seed (1 0) :model-warnings nil :testing-function set-testing-traces))

(defun run-it ()
  (let ((*standard-output* *out-file*))
    (bst-test 2)))

(load-act-r-code "ACT-R:tutorial;lisp;bst.lisp")
