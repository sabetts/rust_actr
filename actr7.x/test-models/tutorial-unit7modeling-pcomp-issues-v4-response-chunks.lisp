
(actr-load "ACT-R:tutorial;lisp;pcomp-issues.lisp")
(actr-load "ACT-R:test-models;models;production-compilation-issues-model-v4.lisp")

(defun filter (evt)
  (when (zerop (evt-time evt))
    (sgp :v nil))
  nil)

(defun run-it () 
 (let ((p (add-pre-event-hook 'filter)))
    (pcomp-issues-trials 2 t nil)
   (delete-event-hook p))
  (sgp :v t)
  (pprint-chunks response-2-0 response-2-1))