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
    (categorize-attribute size -.9)
    (delete-event-hook p))
  (sgp :v t)
  (model-output "? (categorize-attribute size -.9)")
  (model-output ">>> categorize.attribute('size',-.9)")
  (model-output "")
  (buffer-chunk goal))
