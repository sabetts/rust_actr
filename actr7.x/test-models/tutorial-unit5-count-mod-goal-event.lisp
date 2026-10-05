(defvar *set-break*)
(defvar *output*)

(defun filter (evt)
  (when *set-break*
    (sgp :v nil)
    (setf *set-break* nil))
  (when (eq (evt-action evt) 'mod-buffer-chunk)
    (setf *output* (format-event evt)))
  nil)

(defun run-it ()
  (setf *set-break* t)
  
  (let ((p (add-pre-event-hook 'filter)))
    (run .05)
    (delete-event-hook p))
  (sgp-fct (list :v *out-file*))
  (model-output *output*)
  )

(load-act-r-code "ACT-R:tutorial;unit1;count.lisp")
