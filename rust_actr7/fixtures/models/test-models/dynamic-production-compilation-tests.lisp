
(actr-load "ACT-R:test-models;models;dynamic-production-compilation-tests.lisp")

(defun run-it ()
  (let ((p (copy-list (all-productions))))
    
    (dolist (x p)
      (reset)
      
      (when (find x '(g2 i2 p2))
        (set-parameter-value :do-not-harvest nil))
      
      (penable-fct (list x))
      (run .1))))
              