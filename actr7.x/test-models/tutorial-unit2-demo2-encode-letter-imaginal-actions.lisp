
(defvar *demo2-trace-filtering*)
(defvar *demo2-disable-trace*)

(defun filter (evt)
  (when *demo2-disable-trace*
    (setf *demo2-disable-trace* nil)
    (sgp :v nil))
  (when (or (and (eq (evt-action evt) 'production-fired)
                 (eq (production-name (car (evt-params evt))) 'encode-letter))
            (and (eq (evt-action evt) 'module-request)
                 (eq (car (evt-params evt)) 'imaginal))
            (and (eq (evt-action evt) 'clear-buffer)
                 (eq (car (evt-params evt)) 'imaginal))
            (eq (evt-module evt) 'imaginal))
    (when (and (< (evt-time evt) .45) (evt-output evt) (filter-test (get-event-by-id evt)))
      (push-last (format-event evt) *demo2-trace-filtering*))
    nil))

(defun run-it ()
  (setf *demo2-trace-filtering* nil)
  (setf *demo2-disable-trace* t)
  
  (let ((p (add-pre-event-hook 'filter)))
    (ignore-real-time
     (demo2-experiment))
    (delete-event-hook p))
  (sgp :v t)
  
  (model-output "~{~a~^~%...~%~}" *demo2-trace-filtering*))
  
  
  

(actr-load "ACT-R:tutorial;lisp;demo2.lisp")