
(defun run-it () 
  (reset)
  (sgp :ppm 1 :egs 1)
  (sgp :seed (150264085143 2))
  (run 1))
    
(load-act-r-model "ACT-R:tutorial;unit8;simple-ppm-model.lisp")
