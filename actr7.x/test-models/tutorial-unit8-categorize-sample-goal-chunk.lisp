(actr-load "ACT-R:tutorial;lisp;categorize.lisp")

(defvar *count*)

(defun filter (evt)
  (when (zerop (evt-mstime evt))
    (sgp :v nil)
    )
  (if (= (incf *count*) 3)
      "break"
    nil))

(defun run-it ()
  (setf *count* 0)
  (let ((p (add-pre-event-hook 'filter)))
    (present-one-attribute-fct 'eh 0.704)
    (delete-event-hook p))
  (sgp :v t)
  (buffer-chunk goal))