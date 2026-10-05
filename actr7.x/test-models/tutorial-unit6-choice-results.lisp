(load-act-r-code "ACT-R:examples;act-r-solutions;unit-6-choice-solution-task.lisp")

(defvar *starting-seed*)

(defun example-data ()
  (do ((data nil)
       (i 0 (1+ i)))
      ((= i 100) 
       (sgp :v t)
       (apply 'print-results (list (funcall 'analyze data))))
    (reset)
    (sgp-fct (list :seed (list (incf *starting-seed*) 0) :v nil))
    (push (do-n-blocks-of-m-trials 4 12) data)))


(defun run-it ()
  (setf *starting-seed* 105)
  (example-data))
