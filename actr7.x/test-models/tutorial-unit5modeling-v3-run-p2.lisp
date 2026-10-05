(ssp :starting-parameters (:model-warnings nil :testing-function set-testing-traces))

(actr-load "ACT-R:test-models;models;declarative-issues-v3.lisp")

(defvar *disable-trace*)
(defvar *wanted-output*)

(defun filter (evt)
  (when *disable-trace*
    (setf *disable-trace* nil)
    (sgp :v nil)
    )
  (when (> (evt-mstime evt) 12300)
    (sgp :v t))
  nil)

(defun run-it ()
  (setf *disable-trace* t)
  
  (let ((p (add-pre-event-hook 'filter)))
    (sgp :act t)
    (run 19.517)
    (delete-event-hook p)))  

