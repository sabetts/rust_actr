
(defvar *alpha* .2)

(ssp :starting-parameters (:seed (1 0) :model-warnings nil :testing-function paired-learning-params))

(defun paired-learning-params ()
  (set-testing-traces)
  (sgp-fct (list :alpha *alpha*)))

(defun run-it ()
  (setf *alpha* .2)
  (paired-task 2 2)
  (setf *alpha* 1.0)
  (paired-task 1 10))


(load-act-r-code "ACT-R:tutorial;lisp;paired.lisp")
(load-act-r-model "ACT-R:tutorial;unit7;paired-learning-model.lisp")

