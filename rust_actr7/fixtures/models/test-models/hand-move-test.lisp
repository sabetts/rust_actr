
(defun key-event-handler (model key)
  (declare (ignore model))
  (model-output "The key is ~s" key))

(defun run-it ()
  (add-act-r-command "hand-move-test" 'key-event-handler)
  (monitor-act-r-command "output-key" "hand-move-test")
  (install-device '("motor" "cursor" "mouse"))
  (run 10)
  (remove-act-r-command-monitor "output-key" "hand-move-test")
  (remove-act-r-command "hand-move-test"))

(load-act-r-model "ACT-R:test-models;models;hand-move-test.lisp")
