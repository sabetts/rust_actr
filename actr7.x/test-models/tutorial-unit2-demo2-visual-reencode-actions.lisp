
(defvar *demo2-trace-filtering*)
(defvar *demo2-disable-trace*)
(defvar *demo2-trace-started*)

(defun filter (evt)
  (when *demo2-disable-trace*
    (setf *demo2-disable-trace* nil)
    (sgp :v nil))
  (when (and (eq (evt-module evt) :vision)
             (= (evt-mstime evt) 970)
             (evt-output evt) 
             (filter-test (get-event-by-id evt)))
    (push-last (format-event evt) *demo2-trace-filtering*))
  nil)

(defun run-it ()
  (setf *demo2-trace-filtering* nil)
  (setf *demo2-disable-trace* t)
  (setf *demo2-trace-started* nil)
  
  (let ((p (add-pre-event-hook 'filter)))
    (ignore-real-time
     (demo2-experiment))
    (delete-event-hook p))
  (sgp :v t)
  
  (model-output "~{~a~^~%~}" *demo2-trace-filtering*))
  
  
  

(actr-load "ACT-R:tutorial;lisp;demo2.lisp")