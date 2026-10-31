(clear-all)
(define-model safe-bind-output
  (sgp :epl t :pct t :trace-detail low)
  (chunk-type state step val)
  (add-dm (g isa state step one val color))
  (goal-focus g)
  (p bind-a
     =goal>
       step one
       val =s
     !safe-bind! =v 7
     ==>
     =goal>
       step two
       val new)
  (p bind-b
     =goal>
       step two
       val =v
     ==>
     !output! =v
     =goal>
       step done))
