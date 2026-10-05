
(defun run-it () 
  
  (reset)
  (let* ((window (open-exp-window "Letter recognition" :visible nil)))
    
    (add-text-to-exp-window window "x" :x 125 :y 150)
    
    (install-device window)
    (print-visicon)
    (run .01)
    (print-visicon)
    (run .06)
    (let ((new-items (add-visicon-features '(screen-x 50 screen-y 30 color green) '(screen-x 30 screen-y 50 color blue))))
      (print-visicon)
      (run .01)
      (print-visicon)
      (run .02)
      (delete-visicon-features (car new-items))
      (modify-visicon-features (list (second new-items) 'color 'black 'value "changed"))
      (print-visicon)
      (run .01)
      (print-visicon))))
    
    

(actr-load "ACT-R:tutorial;lisp;demo2.lisp")

