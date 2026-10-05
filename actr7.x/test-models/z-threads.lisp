(clear-all)

(ssp :starting-parameters (:seed (1 0) :model-warnings nil :testing-function set-testing-traces))

(defun run-it () 
  (force-virtual-window
   (m))
  (clear-all)
  (undefine-module goal)
  (setf *modules* (remove "threads" *modules* :test 'string-equal))
  (suppress-warnings (load-act-r-code "ACT-R:core-modules;goal.lisp"))
  )

(require-extra "threads")
(actr-load "ACT-R:extras;threads;sample.lisp")
