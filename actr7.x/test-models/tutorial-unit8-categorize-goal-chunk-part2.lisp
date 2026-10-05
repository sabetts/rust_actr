(actr-load "ACT-R:tutorial;lisp;categorize.lisp")

(defvar *set*)

(defun filter (evt)
  (when (zerop (evt-mstime evt))
    (sgp :v nil)
    )
  (unless *set*
    (when (buffer-read 'imaginal)
      (mod-buffer-chunk 'imaginal '(category 1))
      (setf *set* t))))

(defun run-it ()
  (let ((p (add-pre-event-hook 'filter)))
    (setf *set* nil)
    (trial (car *stims*))
    (delete-event-hook p))
  (sgp :v t)

  (buffer-chunk goal))