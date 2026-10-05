(defvar *set-break*)

(defun filter (evt)
  (when *set-break*
    (pbreak retrieve-from-person retrieve-from-location)
    (sgp :v nil :cmdt nil)
    (setf *set-break* nil))
  
  nil)

(defun run-it ()
  (setf *set-break* t)
  
  (let ((p (add-pre-event-hook 'filter)))
    (fan-sentence "lawyer" "store" t 'person)
    (delete-event-hook p))
  (sgp-fct (list :v *out-file* :cmdt *out-file*))
  (buffer-chunk imaginal))

(load-act-r-code "ACT-R:tutorial;lisp;fan.lisp")
