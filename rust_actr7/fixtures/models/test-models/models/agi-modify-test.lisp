(clear-all)

(define-model test (sgp-fct (list :needs-mouse t :v *out-file* :cmdt *out-file*))
  
  (set-visloc-default kind oval screen-x lowest)
  (install-device '("motor" "cursor" "mouse"))
  (start-hand-at-mouse)
  (set-cursor-position 300 300)
  
  (p start
     ?goal>
     buffer empty
     =visual-location>
     ?visual>
     state free
     ?manual>
     state free
     ==>
     +goal>
     
     +manual>
     isa move-cursor
     loc =visual-location
     
     +visual>
     isa move-attention
     screen-pos =visual-location)
  
  (p click
     =visual>
     ?manual>
     state free
     =goal>
     ==>
     !eval! (buffer-chunk visual)
     +manual>
     isa click-mouse)
  
  (p next
     =goal>
     ?manual>
     state free
     ?visual>
     buffer empty
     ==>
     +visual-location>
     kind oval
     > screen-x current
     screen-x lowest
     -goal>
     ))
     
     