
(clear-all)

(define-model test
    (sgp :v t :trace-detail high)
  (install-device (open-exp-window ""))
  (p p1
     ?goal>
     buffer empty
     ?manual>
     state free
     ==>
     +manual>
     isa hand-to-mouse
     +goal>
     )
  (p p2 
     =goal>
     value nil
     ?manual>
     state free
     ==>
     +manual>
     isa hand-to-home
     +goal>
     value 3)
  (p p3
     =goal>
     value 3
     ?manual>
     state free
     ==>
     +goal>
     value 4
     +manual>
     isa hand-to-mouse)
    
  (p p4
     =goal>
     value 4
     ?manual>
     state free
     ==>
     +goal>
     value 5
     +manual>
     isa clear)
  (p p5
     =goal>
     value 5
     ?manual>
     state free
     ==>
     +manual>
     isa hand-to-home
     +goal>
     value 6)
  
  (p p6
     =goal>
     value 6
     ?manual>
     state free
     ==>
     +goal>
     value 7
     +manual>
     isa peck
     hand right
     finger index
     r 1
     theta 1.57)
  
  (p p7
     =goal>
     value 7
     ?manual>
     state free
     ==>
     +manual>
     isa point-hand-at-key
     hand right
     to-key u
     +goal>
     value 8)
  
    (p p8
     =goal>
     value 8
     ?manual>
     state free
     ==>
     +manual>
     isa punch
     hand right
     finger index
     +goal>
     value 9)
  
    (p p9
     =goal>
     value 9
     ?manual>
     state free
     ==>
     +manual>
     isa point-hand-at-key
     hand right
       to-key u
       offsets standard
     +goal>
     value 10)
  
    (p p10
     =goal>
     value 10
     ?manual>
     state free
     ==>
     +manual>
     isa punch
     hand right
     finger index
     +goal>
     value 11)
  
  (p p11
     =goal>
     value 11
     ?manual>
     state free
     ==>
     +manual>
     isa point-hand-at-key
     hand right
       to-key u
       offsets ((index -1 0))
     +goal>
     value 12)
  
    (p p12
     =goal>
     value 12
     ?manual>
     state free
     ==>
     +manual>
     isa punch
     hand right
     finger index
     +goal>
     value 13)
     )
