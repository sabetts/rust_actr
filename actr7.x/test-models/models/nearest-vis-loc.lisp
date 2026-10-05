
(clear-all)

(define-model find-loc
  (sgp :trace-detail high :visual-attention-latency 0)
  
  (define-chunks (start screen-x 0 screen-y 0))
  
  (p find
     ?goal>
     buffer empty
     ==>
     +goal>
     isa chunk
     !bind! =which ("which-one")
     +visual-location>
     isa visual-location
     :nearest =which)
  
  (p report
     =goal>
     isa chunk
     =visual-location>
     isa visual-location
     screen-x =x 
     screen-y =y
     ==>
     !output! (found =visual-location =x =y)))
     
     
