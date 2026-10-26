(defun run-it ()
  (run 10)
  (remove-act-r-command "single-bind")
  (remove-act-r-command "multi-bind"))

(defun single-bind (val)
  (1+ val))

(add-act-r-command "single-bind" 'single-bind "function for testing binding")

(defun multi-bind (chunk)
  (values (chunk-slot-value-fct chunk 'slot1)
          (chunk-slot-value-fct chunk 'slot2)))
          
(add-act-r-command "multi-bind" 'multi-bind "function for testing binding")
     

(load-act-r-model "ACT-R:test-models;models;remote-bind-test.lisp")
