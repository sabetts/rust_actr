
;; Custom trial function to create different but fixed seed values.

(actr-load "ACT-R:tutorial;lisp;siegler.lisp")

(defvar *starting-seed*)

(defun siegler-trial (arg1 arg2)
  (reset)  
  (suppress-warnings (sgp-fct (list :seed (list (incf *starting-seed*) 0) :v nil :mp 5 :ans .5 :rt -10)))
  (set-base-levels 
 (f00 0)(f01 0)(f02 0)(f03 0)(f04 0)
 (f10 0)(f11 0)(f12 0)(f13 0)
 (f20 0)(f21 0)(f22 0)
 (f30 0)(f31 0)
 (f40 0))
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
    (siegler-experiment 100)))
  

