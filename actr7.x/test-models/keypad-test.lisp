(defun key-event-handler (model key)
  (declare (ignore model))
  (model-output "The key is ~s" key))

(defun run-it ()
  (add-act-r-command "keypad-test" 'key-event-handler)
  (monitor-act-r-command "output-key" "keypad-test")
  (run 30)
  (remove-act-r-command-monitor "output-key" "keypad-test")
  (remove-act-r-command "keypad-test"))


(load-act-r-model "ACT-R:test-models;models;keypad-test.lisp")
