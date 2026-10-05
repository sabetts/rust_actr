

(defun run-it ()
  (reset)
  (run 1)
  (reset)
  (sgp :ppm 1)
  (run 1)
  (reset)
  (sgp :seed (100 1) :egs 1 :ppm 1)
  (run 1))

(load-act-r-model "ACT-R:tutorial;unit8;simple-ppm-model.lisp")
