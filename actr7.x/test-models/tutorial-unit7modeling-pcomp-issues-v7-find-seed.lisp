
(actr-load "ACT-R:tutorial;lisp;pcomp-issues.lisp")
(actr-load "ACT-R:test-models;models;production-compilation-issues-model-v7.lisp")

(defvar *count*)

(defun set-flag () (setf *count* t))

(defun filter (evt)
  (when (zerop (evt-time evt))
    (sgp :v nil))
  nil)


(defun run-it ()
  (setf *count* nil)
  (add-act-r-command "set-flag" 'set-flag "")
  (let ((*standard-output* *out-file*))
    (while (null *count*)
      (let ((p (add-pre-event-hook 'filter)))
        (pcomp-issues-game 1)
        (format t "~%")
        (delete-event-hook p))))
  (remove-act-r-command "set-flag"))
  