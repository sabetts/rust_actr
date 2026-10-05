
(actr-load "ACT-R:tutorial;lisp;paired.lisp")

(defvar *set-break*)
(defvar *output*)

(defun filter (evt)
  (unless *output*
    (setf *output* t)
    (sgp :v nil :epl t)
    )
  nil)

(defun run-it ()
  (setf *output* nil)
  (let ((p (add-pre-event-hook 'filter)))
    (paired-task 1 2)
    (delete-event-hook p))
  (sgp :v t)
  (pp production0)
  )
