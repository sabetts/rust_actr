
(defun run-it () 
  (define-model f (sgp :v t))
  (defvar *number* 3)
  (model-output "? (add-dm-fct (list (list 'foo 'size *number*)))")
  (model-output "~s" (add-dm-fct (list (list 'foo 'size *number*))))
  (model-output "? (dm foo)")
  (dm foo))
