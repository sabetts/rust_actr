

(actr-load "ACT-R:tutorial;lisp;past-tense.lisp")

(defvar *buffer-contents*)

(defun get-buffers-contents ()
  (setf *buffer-contents* (printed-buffer-chunk 'goal 'imaginal)))

(defvar *schedule-it*)

(defun filter (evt)
  (when *schedule-it*
    (schedule-event 1 'get-buffers-contents :params nil :time-in-ms t :output t)
    (trigger-reward t)
    (setf *schedule-it* nil))
  nil)

(defun run-it () 
  (sgp :seed (100 1) :model-warnings nil)
  (setf *schedule-it* t)
  (let ((p (add-pre-event-hook 'filter)))
    (past-tense-trials 1 nil nil)
    (delete-event-hook p))
  (sgp :v t)
  (model-output *buffer-contents*))
  
  
