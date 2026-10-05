
(ssp :starting-parameters (:model-warnings nil :seed (1 0) :testing-function set-testing-traces :use-tree t))

(defun run-it ()
  (let ((*standard-output* *out-file*))
    (bst-ppm-experiment 1)))

(load-act-r-code "ACT-R:tutorial;lisp;bst-ppm.lisp")
