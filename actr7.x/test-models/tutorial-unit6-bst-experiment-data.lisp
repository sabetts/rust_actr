

(actr-load "ACT-R:tutorial;lisp;bst.lisp")


; replace the experiment function so that seeds can be set and 
; output sent to *out-file*

(defvar *starting-seed*)

(defun bst-experiment (n &optional human)
  (let ((stims *exp-stims*))
    
    (let ((result (make-list (length stims) :initial-element 0))
          (p-values (list '(decide-over 0) '(decide-under 0) '(force-over 0) '(force-under 0))))
      (dotimes (i n result)
        (reset)
        (sgp-fct (list :seed (list (incf *starting-seed*) 0) :v nil))
        
        (setf result (mapcar '+ result 
                       (mapcar (lambda (x) 
                                 (if (string-equal x "over") 1 0)) 
                         (bst-set human human stims))))
        (no-output
         (setf p-values (mapcar (lambda (x) 
                                  (list (car x) (+ (second x) (production-u-value (car x)))))
                          p-values))))
      
      (setf result (mapcar (lambda (x) (* 100.0 (/ x n))) result))

      (when (= (length result) (length *bst-exp-data*))
        (correlation result *bst-exp-data* *out-file*)
        (mean-deviation result *bst-exp-data* *out-file*))
      
      (format *out-file* "~%Trial ")
      
      (dotimes (i (length result))
        (format *out-file* "~6s" (1+ i)))
      
      (format *out-file* "~%  ~{~6,1f~}~%~%" result)
      
      (dolist (x p-values)
        (format *out-file* "~12s: ~6,4f~%" (car x) (/ (second x) n))))))



(defun run-it ()
  (setf *starting-seed* 100)
  (bst-experiment 100))

