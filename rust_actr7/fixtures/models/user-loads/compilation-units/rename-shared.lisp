(clear-all)
(define-model rename-shared
  (sgp :epl t :pct t :trace-detail low)
  (chunk-type state step val)
  (add-dm (g isa state step one val color))
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
       val =s
     ==>
     =goal>
       step done))
