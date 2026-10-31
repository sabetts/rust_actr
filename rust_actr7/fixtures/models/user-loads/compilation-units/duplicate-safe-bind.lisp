(clear-all)
(define-model duplicate-safe-bind
  (sgp :epl t :pct t :trace-detail low)
  (chunk-type state step val)
  (add-dm (g isa state step one val color))
  (goal-focus g)
  (p bind-a
     =goal>
       step one
       val =s
     !safe-bind! =v 1
     ==>
     =goal>
       step two
       =s =v)
  (p bind-b
     =goal>
       step two
       val =s
     !safe-bind! =v 1
     ==>
     !output! =v
     =goal>
       step done))
