(ssp :starting-parameters (:model-warnings nil :testing-function set-testing-traces))

(actr-load "ACT-R:test-models;models;declarative-issues-v4.lisp")

(defvar *disable-trace*)
(defvar *wanted-output*)

(defun filter (evt)
  (when *disable-trace*
    (setf *disable-trace* nil)
    (sgp :v nil)
    )
  (when (> (evt-mstime evt) 26590)         
    (sgp :v t))
  
  nil)

(defun run-it ()
  (setf *disable-trace* t)
  (sgp :act medium)
  (let ((p (add-pre-event-hook 'filter)))
    (run 26.591)
    (delete-event-hook p)))

