
;; Custom trial function to create different seed values and turn off the trace
;; while using the same model, and have print-results send output to *out-file*.

(actr-load "ACT-R:tutorial;lisp;sperling.lisp")

(defvar *starting-seed*)

(defun sperling-trial (onset-time)
  
  (reset)
  (sgp-fct (list :seed (list (incf *starting-seed*) 0) :v nil))
  
  (let* ((letters (permute-list '("B" "C" "D" "F" "G" "H" "J" 
                                  "K" "L" "M" "N" "P" "Q" "R" 
                                  "S" "T" "V" "W" "X" "Y" "Z")))
         (answers nil)   
         (row (act-r-random 3))
         (window (open-exp-window "Sperling Experiment" :visible t))
         freq)
    
    (dotimes (i 3)
      (dotimes (j 4)        
        (let ((txt (nth (+ j (* i 4)) letters)))
          (when (= i row)
            (push txt answers))
          (add-text-to-exp-window window txt :x (+ 75 (* j 50)) :y (+ 100 (* i 50))))))
 
    (install-device window)
    
    (case row 
      (0 
       (setf freq 2000))
      (1 
       (setf freq 1000))
      (2 
       (setf freq 500)))
    
    (new-tone-sound freq .5 onset-time)
    (schedule-event-relative (+ 900 (act-r-random 200)) "clear-exp-window" :params (list window) :time-in-ms t)
    
    (setf *responses* nil)
    
    (add-act-r-command "sperling-response" 'respond-to-key-press 
                       "Sperling task key press response monitor")
    (monitor-act-r-command "output-key" "sperling-response")
   
    (run 30 nil)
    
    (remove-act-r-command-monitor "output-key" "sperling-response")
    (remove-act-r-command "sperling-response")
    
        
    (sgp-fct (list :v *out-file*))
    (compute-score answers)))


(defun print-results (data)
  
  (format *out-file* "~%Condition    Current Participant   Original Experiment~%")
  (do ((condition '(0.00 0.15 0.30 1.00) (cdr condition))
       (temp1 data (cdr temp1))
       (temp2 *sperling-exp-data* (cdr temp2)))
      ((null temp1))
    (format *out-file* " ~4,2F sec.          ~6,2F                ~6,2F~%" 
              (car condition) (car temp1) (car temp2))))

(defun run-it () 

  (setf *starting-seed* 101)

  (model-output "? (sperling-experiment 100)")
  (sperling-experiment 100)
  ;; Fake running the Python version and just print the command
  (model-output "~%~%>>> sperling.experiment(100)")
  (sperling-experiment 100))
  

