
(defun run-it () 
    (reset)
  (sgp :v nil)
  (run-n-events 2)
  (sgp :v t)
  (buffer-chunk goal))
    
(load-act-r-model "ACT-R:tutorial;unit8;simple-dynamic-model.lisp")