
(defvar *demo-trace-filtering*)
(defvar *demo-disable-trace*)
(defvar *demo-trace-started*)

(defun filter (evt)
  (when *demo-disable-trace*
    (setf *demo-disable-trace* nil)
    (sgp :v nil))
  (when (string-equal (evt-action evt) "clear-exp-window")
    (setf *demo-trace-filtering* (format-event evt)))
  nil)

(defun run-it ()
  (setf *demo-trace-filtering* nil)
  (setf *demo-disable-trace* t)
  (setf *demo-trace-started* nil)
  
  (let ((p (add-pre-event-hook 'filter)))
    (ignore-real-time
     (sperling-trial .15))
    (delete-event-hook p))
  (sgp :v t)
  
  (model-output *demo-trace-filtering*)
  )
  
  
  

(actr-load "ACT-R:tutorial;lisp;sperling.lisp")