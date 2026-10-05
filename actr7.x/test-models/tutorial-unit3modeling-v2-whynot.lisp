
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
     (pm-issues-task "next"))
    (delete-event-hook p))
  (sgp :v t)
  (whynot respond-next))
  
(actr-load "ACT-R:tutorial;lisp;pm-issues.lisp")
(actr-load "ACT-R:test-models;models;perceptual-motor-issues-model-v2.lisp")

