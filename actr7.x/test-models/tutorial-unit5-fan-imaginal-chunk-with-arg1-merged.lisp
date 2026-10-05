(defvar *set-break*)
(defvar *output*)

(defun filter (evt)
  (when *set-break*
    (sgp :v nil)
    (setf *set-break* nil))
  (when (and (= (evt-mstime evt) 300)
             (eq (evt-action evt) 'conflict-resolution))
    (setf *output* (printed-buffer-chunk 'imaginal))
    )
  nil)

(defun run-it ()
  (setf *set-break* t)
  
  (let ((p (add-post-event-hook 'filter)))
    (fan-sentence "lawyer" "store" t 'person)
    (delete-event-hook p))
  (sgp-fct (list :v *out-file*))
  (model-output *output*)
  )

(load-act-r-code "ACT-R:tutorial;lisp;fan.lisp")
