
(actr-load "ACT-R:tutorial;lisp;paired.lisp")
(actr-load "ACT-R:tutorial;unit7;paired-learning-model.lisp")

(defvar *output*)

(defun filter (evt)
  (unless *output*
    (sgp :seed (101 1) :alpha 1 :pct t :v nil)
    (setf *output* t)
    )
  (if (and 
       (= (evt-mstime evt) 260635)
       (eq (evt-action evt) 'production-fired))
      "break"
    nil))

(defun filter2 (evt)
  (if (= *output* 1)
      (sgp :v nil)
    (incf *output*))
  nil)

(defun run-it ()
  (setf *output* nil)
  (let ((p (add-pre-event-hook 'filter)))
    (paired-task 1 27)
    (delete-event-hook p))
  (sgp :v t)
 
  (setf *output* 0)
  (let ((p (add-pre-event-hook 'filter2)))
    (run-n-events 2)
    (delete-event-hook p))
  )
