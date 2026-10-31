(clear-all)
(define-model self-composition
  (sgp :epl t :pct t :trace-detail low)
  (chunk-type state state)
  (add-dm (g isa state state start))
  (goal-focus g)
  (p g1
     =goal>
       state =s
     ==>))
