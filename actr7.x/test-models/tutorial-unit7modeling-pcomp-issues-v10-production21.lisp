
(actr-load "ACT-R:tutorial;lisp;pcomp-issues.lisp")
(actr-load "ACT-R:test-models;models;production-compilation-issues-model-v10.lisp")


(defun filter (evt)
  (when (zerop (evt-time evt))
    (sgp :v nil))
  nil)


(defun run-it ()
  (let ((*standard-output* *out-file*))
    (let ((p (add-pre-event-hook 'filter)))
      (pcomp-issues-trials 6 t nil)
      (delete-event-hook p))
    (sgp :v t)
    (pp production21
        )))