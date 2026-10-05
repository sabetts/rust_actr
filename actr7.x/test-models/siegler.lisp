
(ssp :starting-parameters (:seed (1 0) :testing-function set-testing-traces))

(defun run-it ()
  (let ((*standard-output* *out-file*))
    (siegler-experiment 2)))

(load-act-r-code "ACT-R:tutorial;lisp;siegler.lisp")

