
;; Custom do-experiment function to create different seed values and turn off the trace
;; while using the same model, and have print-results send output to *out-file*.

(actr-load "ACT-R:tutorial;lisp;paired.lisp")

(defvar *starting-seed*)

(defun do-experiment (size trials human)
  
  (if (and human (not (visible-virtuals-available?)))
      (print-warning "Cannot run the task as a person without a visible window available.")
    (progn
      (reset)
      (suppress-warnings
       (sgp-fct (list :seed (list (incf *starting-seed*) 0) :bll .5 :ans nil :lf 1 :rt 0 :v nil)))
     
      (let* ((result nil)
             (model (not human))
             (window (open-exp-window "Paired-Associate Experiment" :visible human)))
        
        (when model
          (install-device window))
        
        (dotimes (i trials) 
          (let ((score 0.0)
                (time 0.0))
            
            (dolist (x (permute-list (subseq *pairs* (- 20 size)))) 
              
              (clear-exp-window window)
              (add-text-to-exp-window window (first x) :x 150 :y 150)
              
              (setf *response* nil)                   
              (let ((start (get-time model)))
                
                (if model
                    (run-full-time 5)
                  (while (< (- (get-time nil) start) 5000)
                    (process-events)))
                
                (when (equal *response* (second x))      
                  (incf score 1.0)    
                  (incf time (- *response-time* start)))
                
                (clear-exp-window window)
                (add-text-to-exp-window window (second x) :x 150 :y 150)
                (setf start (get-time model))
                
                (if model
                    (run-full-time 5)
                  (while (< (- (get-time nil) start) 5000)
                    (process-events)))))
            
            (push (list (/ score size) (if (> score 0) (/ time score 1000.0) 0)) result)))
        
        (reverse result)))))

(defun print-results (predicted data label)
  
  (format *out-file* "~%~A:~%" label)
  (correlation predicted data *out-file* )
  (mean-deviation predicted data *out-file*)
  (format *out-file* "Trial    1       2       3       4       5       6       7       8~%")
  (format *out-file* "     ~{~8,3f~}~%" predicted))
 
(defun run-it () 

  (setf *starting-seed* 101)
 
  (paired-experiment 10))
  

