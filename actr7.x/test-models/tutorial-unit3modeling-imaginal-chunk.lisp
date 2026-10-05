
(defvar *disable-trace*)

(defun filter (evt)
  (when *disable-trace*
    (setf *disable-trace* nil)
    (sgp :v nil)
    (schedule-break 2.37))
  nil)

(defun run-it ()
  (setf *disable-trace* t)
  
  (let ((p (add-pre-event-hook 'filter)))
    (ignore-real-time
     (pm-issues-task "next"))
    (delete-event-hook p))
  (sgp :v t)
  (buffer-chunk imaginal))
  

(actr-load "ACT-R:tutorial;lisp;pm-issues.lisp")
(actr-load "ACT-R:tutorial;unit3;perceptual-motor-issues-model.lisp")

