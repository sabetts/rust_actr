
(actr-load "ACT-R:tutorial;lisp;bst-ppm.lisp")

(defvar *set-seed* t)

(defvar *events* nil)

(defun filter (evt)
  (when (and *set-seed* (zerop (evt-time evt)))
    (sgp :v nil :seed (100 1))
    (setf *set-seed* nil))
  (if (and (eq (evt-action evt) 'production-fired)
           (eq 'encode-line-current (production-name (first (evt-params evt)))))
      "break"
    nil))

(defun filter2 (evt)
  (push-last (format-event evt) *events*))

(defun run-it ()
  (let ((p (add-pre-event-hook 'filter)))
    (setf *set-seed* t)
    (setf *show-actions* nil)
    (ignore-real-time (bst-set nil nil (subseq *no-learn-stims* 0 1) nil))
    (delete-event-hook p))
  (let ((p (add-pre-event-hook 'filter2)))
    (setf *events* nil)
    
    (sgp :v nil)
    (run-n-events 15)
    (delete-event-hook p))
  (sgp :v t)
  (let ((skipped nil))
    
    (mapcar (lambda (x) 
              (if (or (and (search "imaginal" x :test 'string-equal)
                           (not (search "mod-buffer-chunk" x :test 'string-equal)))
                      (search "encode-line-current" x :test 'string-equal))
                  (progn
                    (setf skipped nil)
                    (model-output x))
                (unless skipped
                  (setf skipped t)
                  (model-output "..."))))
      *events*)))
