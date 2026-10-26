
(defun run-it ()
  (reload) ;; This results in an extra warning when using ACL because it prints the load file which doesn't match the reference 
  (set-buffer-chunk 'goal (car (define-chunks (slot2 t))))
  (whynot b)
  (reset)
  (set-buffer-chunk 'goal (car (define-chunks (slot2 t))))
  (whynot b))


(actr-load "ACT-R:test-models;models;undefined-slots.lisp")

