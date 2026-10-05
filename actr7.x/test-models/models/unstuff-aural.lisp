(clear-all)

(define-model unstuff-aural
    
  (sgp :trace-detail high :style-warnings nil)
  
  (set-audloc-default onset highest)
  
  (p modify-it
     =goal>
     =aural-location>
     ==>
     -goal>
     =aural-location>
       kind oval)
  )
     
     
