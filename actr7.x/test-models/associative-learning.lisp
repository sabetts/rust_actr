
(ssp :starting-parameters (:seed (1 0) :model-warnings nil :testing-function set-testing-traces))

(defvar *original-sji-function* (symbol-function 'get-all-chunk-sjis))


(defun run-it () 
  (approach-1-results 1 .75 :prior-p .08 :prior-w 2.5 :show t)
  (clear-all)
  (undefine-module associative-learning)
  (setf *modules* (remove "associative-learning" *modules* :test 'string-equal))
    
  (setf (symbol-function 'get-all-chunk-sjis) *original-sji-function*)
  (setf (symbol-function 'merge-time-lists) (lambda (a b) nil)))

(actr-load "ACT-R:extras;associative-learning;al-fan-study.lisp")
