
(defun run-it () 
  (model-output "? (pm-issues-task \"next\")")
  (model-output "~{~a~^~%~}" (multiple-value-list (ignore-real-time (pm-issues-task "next")))))

(actr-load "ACT-R:tutorial;lisp;pm-issues.lisp")
(actr-load "ACT-R:test-models;models;perceptual-motor-issues-model-v7.lisp")

