

(defun filter (evt)
  (when (zerop (evt-mstime evt))
    (sgp :v nil))
               
  (if (eq (evt-action evt) 'conflict-resolution)
      "break"
    nil))

(defun filter2 (evt)
  (unless (eq (evt-action evt) 'conflict-resolution) 
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
    (run-n-events 2)
    (delete-event-hook p)))
    
    
(load-act-r-model "ACT-R:tutorial;unit8;simple-dynamic-model.lisp")