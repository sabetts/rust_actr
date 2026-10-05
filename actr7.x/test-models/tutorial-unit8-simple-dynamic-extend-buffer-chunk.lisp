

(defun filter (evt)
  (when (zerop (evt-mstime evt))
    (sgp :v nil))
               
  (if (and (eq (evt-action evt) 'production-fired)
           (eq (production-name (first (evt-params evt))) 'retrieve-second-step))
      "break"
    nil))

(defvar *count*)

(defun filter2 (evt)
  (when (> (incf *count*) 3)
      (sgp :v nil))
  nil
  )

(defun run-it () 
  (let ((p (add-pre-event-hook 'filter)))
    (reset)
    (sgp :v nil)
    (run 1)
    (delete-event-hook p))
  
  (sgp :v t)
  
  (let ((p (add-pre-event-hook 'filter2)))
    (setf *count* 0)
    (run-n-events 4)
    (delete-event-hook p)))
    
    
(load-act-r-model "ACT-R:tutorial;unit8;simple-dynamic-model.lisp")