
(clear-all)

(define-model sji-merging
    
    (sgp :trace-detail high :esc t :mas 5.0 :act t :do-not-harvest retrieval :dcnn nil :ga 1.0 :imaginal-activation 0.0)

  (chunk-type source)
  (chunk-type target slot)
  (chunk-type goal state source)
  
  (add-dm (a isa source))
  
  (define-chunks (start) (next) (retrieve-it) (done))
  
  (p p1
     ?goal>
     buffer empty
     ==>
     +goal>
     isa goal
     state start
     +retrieval>
     isa source)
  
  (p p2
     =goal>
     isa goal
     state start
     =retrieval>
     ?imaginal>
     state free
     ==>
     =goal>
     source =retrieval
     state next
     +imaginal>
     isa target
     slot =retrieval)
  
  (p p3
     =goal>
     isa goal
     state next
     - source nil
     =imaginal>
     ==>
     =goal>
     state retrieve-it
     )
  (p p4
     =goal>
     isa goal
     state retrieve-it
     ==>
     =goal>
     state done
     +retrieval>
     isa target
     - slot nil
     ))
     
     
