
(ssp :starting-parameters (:model-warnings nil :testing-function set-testing-traces))

(actr-load "ACT-R:test-models;models;declarative-issues-v6.lisp")


(defun filter (evt)
  (when (and (> (evt-mstime evt) 29980)         
             (eq (evt-action evt) 'production-fired))
    (sgp :v t))
  
  nil)

(defun run-it ()
  (sgp :v nil)
  (let ((p (add-pre-event-hook 'filter)))
    (run 30)
    (delete-event-hook p))
  (model-output "")
  (mp-show-queue))
