(clear-all)

(define-model unstuff-visual
    
    (sgp :trace-detail high :style-warnings nil)
  
  (p modify-it
     =goal>
     =visual-location>
     ==>
     -goal>
     =visual-location>
       color green)
  )
