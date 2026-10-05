
(ssp :starting-parameters (:style-warnings nil :model-warnings nil :seed (1 0) :testing-function set-testing-traces))

(defun run-it ()
  (let ((*standard-output* *out-file*))
    (paired-task 2 2)))

(load-act-r-code "ACT-R:tutorial;lisp;paired.lisp")
(load-act-r-model "ACT-R:tutorial;unit8;paired-dynamic-model.lisp")

