
(defvar *demo2-trace-filtering*)
(defvar *demo2-disable-trace*)

(defun filter (evt)
  (when *demo2-disable-trace*
    (setf *demo2-disable-trace* nil)
    (sgp :v nil))
  (when (and (= (evt-mstime evt) 50)
             (eq (evt-action evt) 'set-buffer-chunk)
             (eq (first (evt-params evt)) 'visual-location)
             )
    (setf *demo2-trace-filtering*
      (printed-buffer-chunk 'visual-location)))
  nil)

(defun run-it ()
  (setf *demo2-trace-filtering* nil)
  (setf *demo2-disable-trace* t)
  (setf *demo2-actions-started* nil)
  
  (let ((p (add-post-event-hook 'filter)))
    (ignore-real-time
     (demo2-experiment))
    (delete-event-hook p))
  
  (sgp :v t)
  
  (model-output *demo2-trace-filtering*))
  
  
  

(actr-load "ACT-R:tutorial;lisp;demo2.lisp")