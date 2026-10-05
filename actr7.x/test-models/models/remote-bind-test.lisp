(clear-all)
(define-model bind-test
    
    (sgp :trace-detail high :style-warnings nil)
  
  (chunk-type goal slot1 slot2)
  
  (p p1
     ?goal>
     buffer empty
     ==>
     +goal>
     isa goal
     slot1 free
     slot2 busy)
  
  (p p2
     =goal>
     isa goal
     !bind! =val ("single-bind" 3)
     !mv-bind! (=val1 =val2) ("multi-bind" =goal)
     ==>
     +goal>
     isa chunk
     !output! (the answers are =val =val1 =val2)
     )
  )
