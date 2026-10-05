
(ssp :starting-parameters (:seed (1 1) :model-warnings nil :testing-function set-testing-traces))

(actr-load "ACT-R:tutorial;lisp;bst.lisp")


(defun filter (evt)
  (when (zerop (evt-time evt))
    (sgp :v nil))
  nil)

(defun run-it ()
  (let ((p (add-pre-event-hook 'filter)))
    (ignore-real-time (bst-test 1 nil))
    (delete-event-hook p))
  (sgp :v t)
  (spp force-over force-under decide-over decide-under)
  )

