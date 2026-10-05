
(defun run-it () 
  (define-model f (sgp :v t))
  (model-output "? (defvar *number* 3)")
  (model-output "~s" (defvar *number* 3))
  (model-output "? (add-dm (foo size *number*))")
  (model-output "~s" (add-dm (foo size *number*)))
  (model-output "? (dm foo)")
  (dm foo))
