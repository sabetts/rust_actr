(defvar *set-break*)
(defvar *output*)

(defun enable-output ()
  (sgp-fct (list :v *out-file* :act 'medium))
  (schedule-event .6 'start-to-stop-output :output nil :maintenance t))

(defun start-to-stop-output ()
  (schedule-event-after-module 'declarative 'stop-output :maintenance t))

(defun stop-output ()
  (sgp :v nil))

(defun filter (evt)
  (when *set-break*
    (sgp :v nil)
    (schedule-event .585 'enable-output :maintenance t :priority -1)
    (setf *set-break* nil)) 
  nil)

(defun run-it ()
  (setf *set-break* t)
  
  (let ((p (add-post-event-hook 'filter)))
    (fan-sentence "giant" "bank" t 'location)
    (delete-event-hook p))
  )

(load-act-r-code "ACT-R:tutorial;lisp;fan.lisp")
