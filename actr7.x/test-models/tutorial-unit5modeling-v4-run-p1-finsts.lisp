(ssp :starting-parameters (:model-warnings nil :testing-function set-testing-traces))

(actr-load "ACT-R:test-models;models;declarative-issues-v4.lisp")

(defvar *disable-trace*)
(defvar *wanted-output*)

(defun filter (evt)
  (when *disable-trace*
    (setf *disable-trace* nil)
    (sgp :v nil)
    )
  (when (and (= (evt-mstime evt) 12259)
             (eq (evt-action evt) 'retrieved-chunk))
         
    (sgp :v t))
  
  nil)

(defun run-it ()
  (setf *disable-trace* t)
  
  (let ((p (add-pre-event-hook 'filter)))
    (run 12.26)
    (delete-event-hook p))
  (model-output "~%? (print-dm-finsts)")
  (print-dm-finsts))

