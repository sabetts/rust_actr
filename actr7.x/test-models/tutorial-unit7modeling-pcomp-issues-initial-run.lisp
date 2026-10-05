
(actr-load "ACT-R:tutorial;lisp;pcomp-issues.lisp")


; Redefine trial fn to use fixed seed

(defvar *initial-seed*)

(defun pcomp-issues-trials (&optional (n 150) (reset t) (output t))
  (when reset (reset))
  (sgp-fct (list :v nil :seed (list (incf *initial-seed*) 0)))
  (setf *window* (open-exp-window "Compilation task" :visible nil))
  (setf *times* nil)
  (setf *responses* nil)
  (setf *task-over* nil)
  (setf *exp-length* n)
  (present-next-trial)
  (install-device *window*)
  
  (add-act-r-command "compilation-issues-response" 'respond-to-key-press "Compilation issues key press response monitor")
  (monitor-act-r-command "output-key" "compilation-issues-response")
  
  ; just run the model a long time to let it finish the trials
  (run 20000)
  
  (remove-act-r-command-monitor "output-key" "compilation-issues-response")
  (remove-act-r-command "compilation-issues-response")

  (analyze-results output))


(defun run-it () 
  (setf *initial-seed* 100)
  (let ((*standard-output* *out-file*))
    (pcomp-issues-game 50 nil)))