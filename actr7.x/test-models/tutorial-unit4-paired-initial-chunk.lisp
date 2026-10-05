
(defvar *disable-trace*)

(defun filter (evt)
  (when *disable-trace*
    (setf *disable-trace* nil)
    (sgp :v nil)
    )
  
  nil)

(defun run-it ()
  (setf *disable-trace* t)
  
  (let ((p (add-pre-event-hook 'filter)))
    (ignore-real-time
     (paired-task 1 1))
    (delete-event-hook p))
  (sgp :v t)
  (sdm - probe nil - answer nil))
  
(actr-load "ACT-R:tutorial;lisp;paired.lisp")

