(actr-load "ACT-R:tutorial;lisp;paired.lisp")
(actr-load "ACT-R:tutorial;unit8;paired-dynamic-model.lisp")

(defun filter (evt)
  (when (zerop (evt-mstime evt))
    (sgp :v nil))
  nil)

(defun run-it ()
  (let ((p (add-pre-event-hook 'filter)))
    (paired-task 1 1)
    (delete-event-hook p))
  (sdm - word nil))