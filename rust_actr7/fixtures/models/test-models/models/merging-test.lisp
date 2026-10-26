
(clear-all)

(define-model foo
  (sgp :trace-detail high :style-warnings nil)
 
  (chunk-type test value)
  (chunk-type value)
  
  (add-dm (a isa value)
          (b isa test value a))
  
  (p p1
     ?goal>
     buffer empty
     ==>
     +goal>
      isa test
     +imaginal>
      isa test
      value a)
  
  (p p2 
     =goal>
      isa test
      value nil
     =imaginal>
      isa test
      value a
     ==>
     =imaginal>
     =goal> 
      value =imaginal)
  
  (p p3
     =goal>
      isa test
      - value b
      - value nil
     =imaginal>
      isa test
     ==>
     -imaginal>)
  
  (p p4
     =goal>
      isa test
      value b
     ==>
     !stop!)
  )
  
