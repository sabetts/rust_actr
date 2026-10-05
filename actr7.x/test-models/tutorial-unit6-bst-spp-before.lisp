
(ssp :starting-parameters (:model-warnings nil :testing-function set-testing-traces))

(actr-load "ACT-R:tutorial;lisp;bst.lisp")

(defun run-it ()
  (model-output "? (spp force-over force-under decide-over decide-under)")
  (model-output "~%>>> actr.spp('force-over', 'force-under', 'decide-over', 'decide-under')~%")
  (spp force-over force-under decide-over decide-under)
  )