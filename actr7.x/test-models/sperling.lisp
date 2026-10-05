
(ssp :starting-parameters (:testing-function test-sperling))

(defvar *act-r-trace-level* 'low)

(defun sperling-print-visicon ()
  (print-visicon)
  (remove-act-r-command-monitor "visicon-update" "sperling-test-visicon-printer"))
  
  
(add-act-r-command "sperling-test-visicon-printer" 'sperling-print-visicon "Testing function to print the visicon for the sperling model. Do not call.")

(defun test-sperling ()
  (set-testing-traces)
  (sgp-fct (list :trace-detail *act-r-trace-level*))
  (monitor-act-r-command "visicon-update" "sperling-test-visicon-printer"))

(defun run-it ()
  (force-virtual-window 
   (ignore-real-time
    (setf *act-r-trace-level* 'low)
    (sperling-trial .15)
    (setf *act-r-trace-level* 'high)
    (sperling-trial .15)))
  (remove-act-r-command "sperling-test-visicon-printer"))

(actr-load "ACT-R:tutorial;lisp;sperling.lisp")

