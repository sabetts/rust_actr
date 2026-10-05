

(actr-load "ACT-R:tutorial;lisp;past-tense.lisp")

(defvar *buffer-contents*)

 
(defun run-it () 
  (sgp :seed (100 1) :model-warnings nil)
;  (schedule-event 1 'set-buffers :params nil :time-in-ms t)
  (past-tense-trials 1 t nil)
  )
  
