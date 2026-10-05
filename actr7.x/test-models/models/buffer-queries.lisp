(clear-all)

(define-model test-buffer-queries
    
  (sgp :trace-detail high :crt t)
  
  
  (add-dm
   (value t))
  
  (p start
     ?goal>
       buffer empty
     ?retrieval>
       buffer empty
       state free
     ==>
     +goal>
     +retrieval>
       value t)
  
  (p full
     ?retrieval>
       buffer full
     ==>
     +retrieval>
       color black)
  
  (p failure
     ?retrieval>
       buffer failure
     ==>))
