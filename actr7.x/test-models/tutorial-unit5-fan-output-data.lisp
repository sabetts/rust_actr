
(defun filter (evt)
  (when (zerop (evt-time evt))
    (sgp :v nil))
  nil)

(defun run-it ()
  (let ((p (add-pre-event-hook 'filter))
        (*standard-output* *out-file*))
    (fan-experiment)
    (delete-event-hook p)))

(load-act-r-code "ACT-R:tutorial;lisp;fan.lisp")
