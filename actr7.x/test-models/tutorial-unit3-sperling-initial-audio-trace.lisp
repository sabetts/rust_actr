
(defvar *demo-trace-filtering*)
(defvar *demo-disable-trace*)
(defvar *demo-trace-started*)

(defun filter (evt)
  (when *demo-disable-trace*
    (setf *demo-disable-trace* nil)
    (sgp :v nil :trace-detail high))
  (if (and
       (<= .135 (evt-time evt) .236)
       (or (eq (evt-module evt) :audio)
           (and (eq (evt-module evt) 'procedural)
                (or (eq (evt-action evt) 'production-selected)
                    (eq (evt-action evt) 'production-fired))))
       (evt-output evt) 
       (filter-test (get-event-by-id evt))
       )
      (push-last (format-event evt) *demo-trace-filtering*)
    (when (and (evt-output evt) 
               (filter-test (get-event-by-id evt))
               (not (string-equal "..." (first (last *demo-trace-filtering*)))))
      (push-last "..." *demo-trace-filtering*))
    )
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
  
  (model-output "~{~a~^~%~}" *demo-trace-filtering*)
  )
  
  
  

(actr-load "ACT-R:tutorial;lisp;sperling.lisp")