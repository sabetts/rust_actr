(define-model indirect-action-compilation
    (sgp :esc t :epl t :pct t :lf 0)
    (add-dm (g value 3 color red))
  
  (goal-focus g)
  
  (p a
     =goal>
     value 3
     ==>
     +retrieval> =goal
     *goal>
     value 2
     )
  
  (p b
     =goal>
     value 2
     =retrieval>
     ==>
     +goal> =retrieval))