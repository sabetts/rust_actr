(defun run-it ()
  (run 10))

(defun single-bind (val)
  (1+ val))

(defun multi-bind (chunk)
  (values (chunk-slot-value-fct chunk 'slot1)
          (chunk-slot-value-fct chunk 'slot2)))
          
     

(load-act-r-model "ACT-R:test-models;models;bind-test.lisp")
