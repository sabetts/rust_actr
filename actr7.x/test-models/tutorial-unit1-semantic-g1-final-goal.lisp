
(defun run-it ()
  (sgp :v nil)
  (run 10)
  (sgp :v t)
  (buffer-chunk goal))

(load-act-r-model "ACT-R:tutorial;unit1;semantic.lisp")