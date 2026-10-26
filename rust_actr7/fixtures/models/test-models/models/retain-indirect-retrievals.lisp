(define-model indirect-action-compilation
    (sgp :esc t :epl t :pct t :lf 0)
  (add-dm (g value 1 color red))
  
  
  (declare-buffer-usage imaginal chunk :all)
  
  (goal-focus g)
  
  (p a
     =goal>
     value 1
     color =c
     ==>
     +retrieval> =c
     =goal>
     value 2
     )
  
  (p b
     =goal>
     value 2
     ==>
     =goal>
     value 3)
  
  (p c
     =goal>
     value 3
     color =c
     ==>
     +retrieval> =c
     =goal>
     value 4)
  (p d
     =goal>
     value 4
     =retrieval>
     ?imaginal>
     state free
     ==>
     =goal>
     value 5
     +imaginal>
     value =retrieval
     )
  (p e
     =goal>
     value 5
     color =c
     ==>
     +retrieval> =c
     =goal>
     value 6)
  (p f
     =goal>
     value 6
     color =c
     =retrieval>
     ==>
     +retrieval> =c
     =goal>
     value 7)
  (p g
     =goal>
     value 7
     =retrieval>
     ==>
     =goal>
     value 8)
  (p h
     =goal>
     value 8
     color =c
     ==>
     +retrieval> =c
     =goal>
     value 9)
  (p i
     =goal>
     value 9
     !safe-bind! =r 'red
     ==>
     +retrieval> =r
     =goal>
     value 10)
  (p j
     =goal>
     value 10
     color =c
     =retrieval>
     ==>
     =goal>
     value 11
     +retrieval> =c)
     
  )