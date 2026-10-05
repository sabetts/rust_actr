
(defun run-it () 
  (model-output "? (pm-issues-task \"next\")")
  (model-output ">>> pm_issues.task('next')")
  (model-output "")
  (ignore-real-time (pm-issues-task "next")))

(actr-load "ACT-R:tutorial;lisp;pm-issues.lisp")
(actr-load "ACT-R:tutorial;unit3;perceptual-motor-issues-model.lisp")

