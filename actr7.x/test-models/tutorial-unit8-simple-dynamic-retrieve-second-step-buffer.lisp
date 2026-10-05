
(defun run-it ()
  (reset)
  (sgp :v nil)
  (pbreak retrieve-second-step)
  (no-output (run 1))
  (sgp :v t)
  (buffer-chunk imaginal))

(load-act-r-model "ACT-R:tutorial;unit8;simple-dynamic-model.lisp")