
(ssp :starting-parameters (:model-warnings nil :testing-function set-testing-traces))

(actr-load "ACT-R:test-models;models;declarative-issues-v1.lisp")

(defun run-it ()
  (sgp :v nil)
  (run 2)
  (sgp-fct (list :v *out-file*))
  (model-output "? (whynot-dm v1 v2 v3)")
  (model-output ">>> actr.whynot_dm('v1','v2','v3')")
  (model-output "")
  (whynot-dm v1 v2 v3))