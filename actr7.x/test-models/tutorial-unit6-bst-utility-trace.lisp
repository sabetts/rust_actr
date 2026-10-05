
(ssp :starting-parameters (:seed (1 1) :model-warnings nil :testing-function set-testing-traces))

(actr-load "ACT-R:tutorial;lisp;bst.lisp")


(defun filter (evt)
  (when (zerop (evt-time evt))
    (sgp :v nil))
  (if (eq (evt-action evt) 'propagate-reward)
    "break"
    nil))

(defun run-it ()
  (let ((p (add-pre-event-hook 'filter)))
    (do-experiment '(15  250  55  125))
    (delete-event-hook p))
  
  (sgp :v t)
  (run-n-events 1)
  )

