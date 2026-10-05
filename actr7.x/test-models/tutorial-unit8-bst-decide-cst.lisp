
(defun filter (evt)
  (when (zerop (evt-mstime evt))
    (sgp :v nil))
               
  (if (and (eq (evt-action evt) 'production-fired)
           (eq (production-name (first (evt-params evt))) 'encode-line-goal))
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
    (sgp :seed (100 1))
    (ignore-real-time (force-virtual-window (do-experiment '(15 250 55 125))))
    (delete-event-hook p))
  (run-n-events 4)
  (sgp :v t :cst t :egs 0)
  (let ((p (add-pre-event-hook 'filter2)))
    (run-n-events 2)
    (delete-event-hook p)))
    
(load-act-r-code "ACT-R:tutorial;lisp;bst-ppm.lisp")
