
(defvar *imaginal-chunk*)

(defun filter (evt)
  (when (zerop (evt-mstime evt))
    (sgp :v nil))
               
  (when (and (eq (evt-action evt) 'production-selected)
             (or (eq (production-name (first (evt-params evt))) 'decide-over)
                 (eq (production-name (first (evt-params evt))) 'decide-under))
             (null *imaginal-chunk*))
    (setf *imaginal-chunk* (printed-buffer-chunk 'imaginal)))
  nil)

(defun run-it () 
  (let ((p (add-pre-event-hook 'filter)))
    (reset)
    (sgp :seed (100 1))
    (setf *imaginal-chunk* nil)
    (ignore-real-time (force-virtual-window (do-experiment '(15 250 55 125))))
    (delete-event-hook p)
    (sgp :v t)
    (model-output *imaginal-chunk*)))
  
(load-act-r-code "ACT-R:tutorial;lisp;bst-ppm.lisp")
