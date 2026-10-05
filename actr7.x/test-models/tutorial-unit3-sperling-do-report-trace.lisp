
(defvar *demo-trace-filtering*)
(defvar *demo-disable-trace*)
(defvar *demo-trace-started*)

(defun filter (evt)
  (when *demo-disable-trace*
    (setf *demo-disable-trace* nil)
    (sgp :v nil))
  (if (and 
       (or (eq *demo-trace-started* :collect)
           (and (null *demo-trace-started*)
                (eq (evt-action evt) 'production-fired)
                (eq (production-name (first (evt-params evt))) 'do-report)
                ))
       (evt-output evt) 
       (filter-test (get-event-by-id evt))
       )
      (progn
        (push-last (format-event evt) *demo-trace-filtering*)
        (cond ((null *demo-trace-started*) (setf *demo-trace-started* :collect))
              ((and (eq *demo-trace-started* :collect)
                    (eq (evt-action evt) 'production-fired))
               (setf *demo-trace-started* :done))))
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