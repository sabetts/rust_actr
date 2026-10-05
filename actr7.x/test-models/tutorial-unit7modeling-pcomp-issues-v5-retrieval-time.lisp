
(actr-load "ACT-R:tutorial;lisp;pcomp-issues.lisp")
(actr-load "ACT-R:test-models;models;production-compilation-issues-model-v5.lisp")

(defvar *count*)

(defun filter (evt)
  (when (zerop (evt-time evt))
    (sgp :v nil)
    )
  (if (eq (evt-action evt) 'production-fired)
      (when (= (incf *count*) 40)
        "break")
    nil))

(defun filter2 (evt)
  (when (= (incf *count*) 12)
    (sgp :v nil)))

(defun run-it ()
  (setf *count* 0)
  (let ((p (add-pre-event-hook 'filter)))
    (pcomp-issues-trials 5 t nil)
    (delete-event-hook p))
  (sgp :v t)
  (setf *count* 0)
  (let ((p (add-pre-event-hook 'filter2)))
    (run-n-events 13)
    (delete-event-hook p)))