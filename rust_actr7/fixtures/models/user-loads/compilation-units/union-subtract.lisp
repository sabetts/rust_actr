(clear-all)
(define-model union-subtract
  (sgp :epl t :pct t :trace-detail low)
  (chunk-type state step val extra)
  (add-dm (g isa state step one val old extra keep))
  (goal-focus g)
  (p union-a
     =goal>
       step one
       val old
     ==>
     =goal>
       step two
       val new)
  (p union-b
     =goal>
       step two
       val new
       extra keep
     ==>
     =goal>
       step done
       extra kept))
