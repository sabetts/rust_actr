
(ssp :starting-parameters (:seed (1 0) :randomize-time t :vpft t :vidt t :testing-function set-testing-traces))

(defun run-it () 
  (force-virtual-window
   (ignore-real-time
    (demo2-experiment))))

(actr-load "ACT-R:tutorial;lisp;demo2.lisp")
