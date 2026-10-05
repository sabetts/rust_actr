
(defun run-it ()
  (reset)
  (run 8.1)
  (sgp :record-ticks nil)
  (run 10))

(load-act-r-model "ACT-R:test-models;models;test-temporal.lisp")
                  
