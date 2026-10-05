
(defun run-it ()
  (model-output "? (goal-focus g2)~%~a" (goal-focus g2))
  (model-output "? (run 1)")
  (model-output "~{~a~^~%~}" (multiple-value-list (run 1)))
  (model-output "? (goal-focus g2)~%~a" (goal-focus g2))
  (model-output "? (run 1)")
  (run 1))

(load-act-r-model "ACT-R:tutorial;unit8;semantic-dynamic-model.lisp")