
(defvar *demo2-trace-filtering*)
(defvar *demo2-disable-trace*)
(defvar *demo2-actions-started*)

(defun filter (evt)
  (when *demo2-disable-trace*
    (setf *demo2-disable-trace* nil)
    (sgp :v nil))
  (when (and (= (evt-mstime evt) 50)
             (or (and (eq (evt-action evt) 'production-fired)
                      (eq (production-name (car (evt-params evt))) 'find-unattended-letter)
                      (setf *demo2-actions-started* t))
                 (and *demo2-actions-started*
                      (eq (evt-action evt) 'conflict-resolution)
                      (setf *demo2-actions-started* nil))
                 *demo2-actions-started*)
             (evt-output evt)
             (filter-test (get-event-by-id evt))
             )
    (push-last (format-event evt) *demo2-trace-filtering*))
  nil)

(defun run-it ()
  (setf *demo2-trace-filtering* nil)
  (setf *demo2-disable-trace* t)
  (setf *demo2-actions-started* nil)
  
  (let ((p (add-pre-event-hook 'filter)))
    (ignore-real-time
     (demo2-experiment))
    (delete-event-hook p))
  
  (sgp :v t)
  
  (model-output "~{~a~^~%~}" *demo2-trace-filtering*))
  
  
  

(actr-load "ACT-R:tutorial;lisp;demo2.lisp")