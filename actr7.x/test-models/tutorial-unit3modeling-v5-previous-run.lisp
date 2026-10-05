
(defun run-it () 
  (model-output "? (pm-issues-task \"previous\")")
  (model-output "~{~a~^~%~}" (multiple-value-list (ignore-real-time (pm-issues-task "previous")))))

(actr-load "ACT-R:tutorial;lisp;pm-issues.lisp")
(actr-load "ACT-R:test-models;models;perceptual-motor-issues-model-v5.lisp")

