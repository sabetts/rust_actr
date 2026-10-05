
(ssp :starting-parameters (:model-warnings nil :testing-function set-testing-traces))

(actr-load "ACT-R:test-models;models;declarative-issues-v5.lisp")


(defun filter (evt)
  (when (and (> (evt-mstime evt) 26590)         
             (eq (evt-action evt) 'clear-buffer))
    (sgp :v t))
  
  nil)

(defun run-it ()
  (sgp :v nil)
  (let ((p (add-pre-event-hook 'filter)))
    (run 28)
    (delete-event-hook p))
  (model-output "")
  (mp-show-queue))
