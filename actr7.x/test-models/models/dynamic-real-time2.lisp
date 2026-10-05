
(clear-all)


(define-model test-model
    
    (sgp :esc t :lf 1.0 :rt 0 :trace-detail medium)
  
  (p start
     ?goal>
     buffer empty
     ==>
     +goal>
     isa chunk
     +retrieval>
     isa chunk)
  
)
