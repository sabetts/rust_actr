
(defun run-it ()
  (pp)
  (ssp :starting-parameters (:do-not-query imaginal :testing-function set-testing-traces))
  (reset)
  (pp))


(load-act-r-model "ACT-R:test-models;models;safe-queries.lisp")

