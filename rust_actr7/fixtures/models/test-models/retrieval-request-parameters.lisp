
(actr-load "ACT-R:test-models;models;retrieval-request-parameters.lisp")

(defun run-it ()
  (let ((p (copy-list (all-productions))))
    
    (dolist (x p)
      (reset)
      
      (penable-fct (list x))
      (run .1))))
              