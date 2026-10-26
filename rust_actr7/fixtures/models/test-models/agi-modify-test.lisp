(defun run-it (&optional visible virtual)
  (reset)
  
  (install-device (open-exp-window "" :visible visible))
  (let ((text1 (create-text-for-exp-window nil "A" :x 20 :y 20 :color 'blue :height 30 :width 40 :font-size 24))
        (text2 (create-text-for-exp-window nil "B" :x 60 :y 30 :color 'red :height 40 :width 30 :font-size 14))
        (text3 (create-text-for-exp-window nil "C" :x 100 :y 50 :color 'green :height 50 :width 20 :font-size 8))
        (button1 (create-button-for-exp-window nil :text "A" :x 20 :y 100 :action (lambda () (model-output "Pressed B1")) :height 20 :width 30 :color 'blue))
        (button2 (create-button-for-exp-window nil :text "B" :x 60 :y 110 :action (lambda () (model-output "Pressed B2")) :height 30 :width 40 :color 'red))
        (button3 (create-button-for-exp-window nil :text "C" :x 100 :y 120 :action (lambda () (model-output "Pressed B3")) :height 40 :width 50 :color 'green))
        (line1 (create-line-for-exp-window nil '(20 160) '(55 180) 'blue))
        (line2 (create-line-for-exp-window nil '(60 180) '(95 150) 'red))
        (line3 (create-line-for-exp-window nil '(130 170) '(100 180) 'green))
        )
    (add-items-to-exp-window nil text1 text2 text3 button1 button2 button3 line1 line2 line3)
    
    (run-n-events 4)
    (print-visicon)
    (pprint-chunks-fct (mapcar 'chunk-visual-loc (visicon-chunks (get-module :vision))))
    (run 10 visible)
    
    (remove-items-from-exp-window nil text1 button2 line3)
    
    (setf text1 (modify-text-for-exp-window text1 :text "D" :x 15 :y 10 :color 'gray :height 20 :width 30 :font-size 20))
    (setf text2 (modify-text-for-exp-window text2))
    (setf text3 (modify-text-for-exp-window text3 :text "E" :x 110 :y 40 :color 'brown :height 30 :width 30 :font-size 14))
    (setf button1 (modify-button-for-exp-window button1 :text "DDD" :x 15 :y 105 :action (lambda () (model-output "Pressed B1m1")) :height 25 :width 35 :color 'gray))
    (setf button2 (modify-button-for-exp-window button2))
    (setf button3 (modify-button-for-exp-window button3 :text "E" :x 115 :y 115 :action (lambda () (model-output "Pressed B3m1")) :height 35 :width 45 :color 'brown))
    (setf line1 (modify-line-for-exp-window line1 '(15 180) '(45 160) 'gray))
    (setf line2 (modify-line-for-exp-window line2 nil nil))
    (setf line3 (modify-line-for-exp-window line3 '(140 190) '(110 165) 'brown))
    
    (add-items-to-exp-window nil text1 button2 line3)
    
    (run-n-events 4)
    
    (print-visicon)
    (pprint-chunks-fct (mapcar 'chunk-visual-loc (visicon-chunks (get-module :vision))))
    (run 10 visible)
    
    (remove-items-from-exp-window nil text3 button3 line1)
    
    (setf text1 (modify-text-for-exp-window text1 :text "F" :x 18 :y 15 :color 'pink :height 10 :width 20 :font-size 10))
    (setf text2 (modify-text-for-exp-window text2 :text "G" :x 65 :y 20 :color 'yellow :height 25 :width 35 :font-size 22))
    (setf text3 (modify-text-for-exp-window text3))
    (setf button1 (modify-button-for-exp-window button1))
    (setf button2 (modify-button-for-exp-window button2 :text "F" :x 55 :y 125 :action (lambda () (model-output "Pressed B2m2")) :height 35 :width 30 :color 'yellow))
    (setf button3 (modify-button-for-exp-window button3 :text "GGG" :x 112 :y 110 :action (lambda () (model-output "Pressed B3m2")) :height 45 :width 25 :color 'pink))
    (setf line1 (modify-line-for-exp-window line1 nil nil))
    (setf line2 (modify-line-for-exp-window line2 '(65 160) '(85 190) 'yellow))
    (setf line3 (modify-line-for-exp-window line3 '(112 175) '(145 193) 'pink))
    
    (add-items-to-exp-window nil text3 button3 line1)
        
    (run-n-events 4)
    
    (print-visicon)
    (pprint-chunks-fct (mapcar 'chunk-visual-loc (visicon-chunks (get-module :vision))))
    (run 10 visible)))


(load-act-r-model "ACT-R:test-models;models;agi-modify-test.lisp")

