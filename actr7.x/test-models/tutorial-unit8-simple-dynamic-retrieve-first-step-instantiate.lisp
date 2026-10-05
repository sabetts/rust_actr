
(defun run-it ()
  (reset)
  (sgp :v nil)
  (pbreak retrieve-first-step)
  (run 1))

(load-act-r-model "ACT-R:tutorial;unit8;simple-dynamic-model.lisp")