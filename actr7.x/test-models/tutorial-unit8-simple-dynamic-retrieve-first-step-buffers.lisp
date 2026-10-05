
(defun run-it ()
  (reset)
  (sgp :v nil)
  (pbreak retrieve-first-step)
  (no-output (run 1))
  (sgp :v t)
  (buffer-chunk goal imaginal retrieval))

(load-act-r-model "ACT-R:tutorial;unit8;simple-dynamic-model.lisp")