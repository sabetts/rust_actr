(defvar *set-break*)
(defvar *output*)

(defun filter (evt)
  (when (or (eq (evt-action evt) 'mod-buffer-chunk)
            (eq (evt-action evt) 'module-mod-request))
    (push-last (format-event evt) *output*))
  nil)

(defun run-it ()
  (setf *output* nil)
  (sgp :v nil)
  (suppress-warnings 
   (p start =goal> start =n count nil ==> *goal> count =n))
  (let ((p (add-pre-event-hook 'filter)))
    (run .05)
    (delete-event-hook p))
  (sgp-fct (list :v *out-file*))
  (model-output "~{~a~^~%~}" *output*)
  )

(load-act-r-code "ACT-R:tutorial;unit1;count.lisp")
