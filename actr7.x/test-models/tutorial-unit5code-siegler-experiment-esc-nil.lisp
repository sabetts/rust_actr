
;; Custom trial function to create different but fixed seed values.

(actr-load "ACT-R:tutorial;lisp;siegler.lisp")

(defvar *starting-seed*)

(defun siegler-trial (arg1 arg2)
  (reset)  
  (suppress-warnings (sgp-fct (list :seed (list (incf *starting-seed*) 0) :v nil :esc nil)))
  (install-device (list "speech" "microphone"))
  (let ((need-to-remove (add-speech-monitor)))
    (new-digit-sound arg1)
    (new-digit-sound arg2 .75)
    (setf *response* nil)
    (run 30)
    (when need-to-remove 
      (remove-speech-monitor))
  *response*))

(defun run-it () 
  (setf *starting-seed* 101)
  (let ((*standard-output* *out-file*))
    (siegler-experiment 1)))
  

