(clear-all)
(define-model retrieval-instantiate
  (sgp :epl t :pct t :trace-detail low)
  (chunk-type state step)
  (chunk-type ans key)
  (add-dm (g isa state step one)
          (resp isa ans key "d"))
  (goal-focus g)
  (p retrieval-a
     =goal>
       step one
     ==>
     +retrieval>
       isa ans
       key "d")
  (p retrieval-b
     =goal>
       step one
     =retrieval>
       isa ans
       key =key
     ==>
     !output! =key
     =goal>
       step done)
  (spp retrieval-b :u 10))
