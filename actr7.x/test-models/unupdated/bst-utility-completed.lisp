
(ssp :starting-parameters (:seed (1 0) :model-warnings nil :testing-function set-testing-traces))

(defun run-it ()
  (let ((*standard-output* *out-file*))
    (bst-experiment 1)))

(load-act-r-code "ACT-R:tutorial;lisp;bst.lisp")
(load-act-r-model "ACT-R:test-models;models;bst-complete-model.lisp")
