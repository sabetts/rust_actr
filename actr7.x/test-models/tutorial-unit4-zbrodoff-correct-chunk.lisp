
(actr-load "ACT-R:tutorial;lisp;zbrodoff.lisp")


(defun run-it ()
  (sgp :v nil)
  (zbrodoff-problem "a" "2" "c" t)
  (sgp-fct (list  :v *out-file*))
  (sdm result c))