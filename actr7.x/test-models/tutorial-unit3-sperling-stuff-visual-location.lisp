
(defvar *demo3-trace-filtering*)
(defvar *demo3-disable-trace*)
(defvar *demo3-trace-started*)

(defun filter (evt)
  (when *demo3-disable-trace*
    (setf *demo3-disable-trace* nil)
    (sgp :v nil))
  (when (and
         (eq (evt-action evt) 'set-buffer-chunk)
         (eq (first (evt-params evt)) 'visual-location)
         (not *demo3-trace-started*)
         )
    (setf *demo3-trace-started* t)
    (push-last (format-event evt) *demo3-trace-filtering*)
    )
  nil)

(defun run-it ()
  (setf *demo3-trace-filtering* nil)
  (setf *demo3-disable-trace* t)
  (setf *demo3-trace-started* nil)
  
  (let ((p (add-pre-event-hook 'filter)))
    (ignore-real-time
     (sperling-trial .15))
    (delete-event-hook p))
  (sgp :v t)
  
  (model-output "~{~a~^~%~}" *demo3-trace-filtering*))
  

(actr-load "ACT-R:tutorial;lisp;sperling.lisp")