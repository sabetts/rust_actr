
(actr-load "ACT-R:tutorial;lisp;pcomp-issues.lisp")
(actr-load "ACT-R:test-models;models;production-compilation-issues-model-v8.lisp")


(defun filter (evt)
  (when (zerop (evt-time evt))
    
    (sgp :v nil)
    )
  (if (and (eq (evt-action evt) 'production-fired)
           (eq (production-name (first (evt-params evt))) 'retrieved-a-non-win))
      "break"
    nil))

(defun filter2 (evt)
  (if (eq (evt-action evt) 'production-fired)
      nil
    (sgp :v nil)))


(defun set-flag () )

(defun run-it ()
  (add-act-r-command "set-flag" 'set-flag "")
  
  (let ((p (add-pre-event-hook 'filter)))
    (pcomp-issues-game 1)
    (delete-event-hook p))
  (sgp :v t)
  (let ((p (add-pre-event-hook 'filter2)))
    (run-n-events 2)
    (delete-event-hook p))
  
  (remove-act-r-command "set-flag"))