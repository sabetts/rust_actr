
(defvar *starting-seed*)

(load-act-r-code "ACT-R:tutorial;lisp;onehit.lisp")

(defun onehit-learning (n &optional (graph t) (game 'game0))
  (let ((data nil)
        (need-to-remove (add-key-monitor)))
    (dotimes (i n)
      (reset)
      (sgp-fct (list :seed (list (incf *starting-seed*) 0) :v nil))
      (funcall game)
      (if (null data)
          (setf data (onehit-blocks 20 5))
        (setf data (mapcar (lambda (x y) 
                             (mapcar '+ x y)) 
                     data (onehit-blocks 20 5)))))
    
    (when need-to-remove
      (remove-key-monitor))
    
    (let ((percentages (mapcar (lambda (x) (/ (car x) n 5.0)) data)))
      (when graph
        (draw-graph percentages))
      
      (list (list (/ (apply '+ (subseq percentages 0 5)) 5)
                  (/ (apply '+ (subseq percentages 5 10)) 5)
                  (/ (apply '+ (subseq percentages 10 15)) 5)
                  (/ (apply '+ (subseq percentages 15 20)) 5))
                  percentages))))


(defun run-it ()
  (setf *starting-seed* 101)
  (let ((x (onehit-learning 50)))
    (sgp-fct (list :v *out-file*))
    (model-output "? (onehit-learning 50)~%~s" x)))
    


