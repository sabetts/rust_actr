(clear-all)
(define-model goal-dynamic-roundtrip
  (sgp :epl t :pct t :trace-detail low)
  (chunk-type state step val)
  (add-dm (g isa state step one val color))
  (goal-focus g)
  (p dynamic-a
     =goal>
       step one
       val =s
     !safe-bind! =v 1
     ==>
     =goal>
       step two
       =s =v)
  (p dynamic-b
     =goal>
       step two
       val =s
       =s =y
     ==>
     !output! =y
     =goal>
       step done))
