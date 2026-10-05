;;; This model runs some simple tests with procedural partial matching.


(defun run-it ()
  (reset)
  (sgp :ppm nil)
  (model-output "~%~30,,,'*<~>~%Without ppm (default symbolic matching):~%~30,,,'*<~>~%")
  (whynot)
  (run 10)
  (reset)
  (sgp :ppm 1)
  (model-output "~%~30,,,'*<~>~%With ppm enabled:~%~30,,,'*<~>~%")
  (whynot)
  (run 10)
  (reset)
  (sgp :ppm 1 :ppm-hook show-ppm-values)
  (model-output "~%~30,,,'*<~>~%With ppm enabled and using the hook to show values as it goes:~%~30,,,'*<~>~%")
  (run 10))

(defun show-ppm-values (p mismatches)
  (model-output "ppm hook called with values:~%~S ~S" p mismatches)
  (fifth (car mismatches)))


(load-act-r-model "ACT-R:test-models;models;test-ppm.lisp")
