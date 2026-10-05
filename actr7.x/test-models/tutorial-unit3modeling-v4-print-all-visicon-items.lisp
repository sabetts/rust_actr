
(defvar *disable-trace*)
(defvar *break-time*)

(defun filter (evt)
  (when *disable-trace*
    (setf *disable-trace* nil)
    (sgp :v nil)
    (when *break-time*
      (schedule-break *break-time*))
    )
  nil)

(defun run-it ()
  
  (setf *disable-trace* t)
  (setf *break-time* .05)
  (let ((p (add-pre-event-hook 'filter)))
    (ignore-real-time
     (pm-issues-task "previous"))
    (delete-event-hook p))
  (sgp :v t)
  (print-visicon)
  
  (setf *disable-trace* t)
  (setf *break-time* 2.3)
  
  (let ((p (add-pre-event-hook 'filter)))
    (ignore-real-time
     (pm-issues-task "next"))
    (delete-event-hook p))
  (sgp :v t)
  (print-visicon)
  
  
  (setf *disable-trace* t)
  (setf *break-time* nil)
  
  (let ((p (add-pre-event-hook 'filter)))
    (ignore-real-time
     (pm-issues-task "previous"))
    (delete-event-hook p))
  (sgp :v t)
  (print-visicon))
  
(actr-load "ACT-R:tutorial;lisp;pm-issues.lisp")
(actr-load "ACT-R:test-models;models;perceptual-motor-issues-model-v4.lisp")

