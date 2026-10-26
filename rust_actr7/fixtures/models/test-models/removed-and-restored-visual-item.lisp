;;; This tests the agi functions for adding and removing
;;; items from an experiment window.


(defvar *response* nil)
(defvar *letter* nil)

(defun key-event-handler (model key)
  (print-visicon)
  (setf *response* key)
  (remove-items-from-exp-window nil *letter*)
  (schedule-event-relative .15 'redisplay-item))

(defun redisplay-item ()
  (print-visicon)
  (add-items-to-exp-window nil *letter*)
  (schedule-event-now 'print-visicon :maintenance t :priority :min))



(defun run-it ()
  
  (add-act-r-command "hand-move-test" 'key-event-handler)
  (monitor-act-r-command "output-key" "hand-move-test")
  
  (reset)
  
  (let* ((lis (permute-list '("B" "C" "D" "F" "G" "H" 
                              "J" "K" "L" "M" "N" "P" 
                              "Q" "R" "S" "T" "V" "W" 
                              "X" "Y" "Z")))
         (text1 (first lis))
         (window (open-exp-window "Letter recognition")))
    
    (setf *letter* (add-text-to-exp-window nil "a" :x 125 :y 150))
    
    (setf *response* nil) 
    (install-device window)
    (run-n-events 3)
    (print-visicon)
    (run 10)
    (print-visicon)
    (buffer-chunk visual)
    
    (reset)
    (clear-exp-window)
    (setf *letter* (add-text-to-exp-window nil "a" :x 125 :y 150))
    (add-text-to-exp-window nil "b" :x 150 :y 150)
    (install-device window)
    (run-n-events 3)
    (print-visicon)
    (run 10)
    (print-visicon)
    (buffer-chunk visual))
  
  (remove-act-r-command-monitor "output-key" "hand-move-test")
  (remove-act-r-command "hand-move-test"))

(load-act-r-model "ACT-R:test-models;models;removed-and-restored-visual-item.lisp")
