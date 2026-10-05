
(defvar *demo3-trace-filtering*)
(defvar *demo3-disable-trace*)
(defvar *demo3-trace-started*)

(defun filter (evt)
  (when *demo3-disable-trace*
    (setf *demo3-disable-trace* nil)
    (sgp :v nil)
    (schedule-break .05))
  nil)

(defun run-it ()
  (setf *demo3-trace-filtering* nil)
  (setf *demo3-disable-trace* t)
  
  (let ((p (add-pre-event-hook 'filter)))
    (ignore-real-time
     (sperling-trial .15))
    (delete-event-hook p))
  (sgp :v t)
  (print-visicon))
  
  
  

(actr-load "ACT-R:tutorial;lisp;sperling.lisp")