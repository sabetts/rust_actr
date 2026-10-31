(clear-all)
(define-model rename-different
  (sgp :epl t :pct t :trace-detail low)
  (chunk-type state step val other)
  (add-dm (g isa state step one val color other color))
  (goal-focus g)
  (p rename-a
     =goal>
       step one
       val =s
     ==>
     =goal>
       step two)
  (p rename-b
     =goal>
       step two
       other =s
     ==>
     =goal>
       step done))
