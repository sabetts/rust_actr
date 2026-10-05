
(defvar *demo2-trace-filtering*)
(defvar *demo2-disable-trace*)
(defvar *demo2-trace-started*)

(defun filter (evt)
  (when *demo2-disable-trace*
    (setf *demo2-disable-trace* nil)
    (sgp :v nil))
  (if (and
       (or (and (eq (evt-action evt) 'production-fired)
                (eq (production-name (car (evt-params evt))) 'attend-letter)
                (setf *demo2-trace-started* t))
           (and (eq (evt-action evt) 'module-request)
                (eq (car (evt-params evt)) 'visual))
           (and (eq (evt-action evt) 'clear-buffer)
                (eq (car (evt-params evt)) 'visual))
           (eq (evt-module evt) :vision))
       (evt-output evt) 
       (filter-test (get-event-by-id evt)))
      (when (and (<= .1 (evt-time evt) .185))
        (push-last (format-event evt) *demo2-trace-filtering*))
    (when (and *demo2-trace-started*
               (not (string-equal "..." (car (last *demo2-trace-filtering*)))))
      (push-last "..." *demo2-trace-filtering*)))
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
  
  (model-output "~{~a~^~%~}" (butlast *demo2-trace-filtering*)))
  
  
  

(actr-load "ACT-R:tutorial;lisp;demo2.lisp")