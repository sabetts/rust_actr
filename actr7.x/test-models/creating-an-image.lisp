

(defun click-brain (text pos)
  (model-output "Clicked image ~s at ~D ~D" text (first pos) (second pos)))

(defun click-brain-2 (value)
  (model-output "Clicked the second brain image given value ~s" value))


(defun run-it ()
  (reset)
  
  (add-act-r-command "click-brain" 'click-brain)
  (add-act-r-command "click-brain-2" 'click-brain-2)
  
  (let ((win (open-exp-window "image test" :visible nil :width 310 :height 420)))
    
    (install-device win)
    
    (start-hand-at-mouse)
    
    (add-image-to-exp-window win "logo" "smalllogo.gif" :x 10 :y 10 :width 288 :height 142)
    
    (add-items-to-exp-window win (create-image-for-exp-window win "brain" "ref-brain.gif" :x 10 :y 160 :width 128 :height 128 :action "click-brain"))
    
    (add-image-to-exp-window win "brain-2" "ref-brain.gif" :x 10 :y 290 :width 128 :height 128 :action (list "click-brain-2" "this string"))
    
    (run-n-events 2) 
    
    (print-visicon)
    
    (run 5))
  
  (remove-act-r-command "click-brain")
  (remove-act-r-command "click-brain-2"))


(load-act-r-model "ACT-R:test-models;models;creating-an-image.lisp")
