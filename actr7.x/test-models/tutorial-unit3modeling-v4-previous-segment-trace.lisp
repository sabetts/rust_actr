
(defvar *disable-trace*)
(defvar *wanted-output*)

(defun filter (evt)
  (when *disable-trace*
    (setf *disable-trace* nil)
    (sgp :v nil)
    )
  (when (and (>= (evt-mstime evt) 2285)
             (or (< (evt-mstime evt) 2420)
                 (and (= (evt-mstime evt) 2420)
                      (eq (evt-action evt) 'production-fired)))
             (evt-output evt)
             (filter-test (get-event-by-id evt)))
    (push-last (format-event evt) *wanted-output*))
  nil)

(defun run-it ()
  (setf *disable-trace* t)
  (setf *wanted-output* nil)
  
  (let ((p (add-pre-event-hook 'filter)))
    (ignore-real-time
     (pm-issues-task "previous"))
    (delete-event-hook p))
  (sgp :v t)
  (model-output "~{~a~^~%~}" *wanted-output*))
  
(actr-load "ACT-R:tutorial;lisp;pm-issues.lisp")
(actr-load "ACT-R:test-models;models;perceptual-motor-issues-model-v4.lisp")

