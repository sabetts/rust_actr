(clear-all)

(define-model probe-compile-utility
  (sgp :seed (100 0) :v nil :esc t :ul t :ult t :egs 0 :alpha .2
       :epl t :pct t :trace-detail high)

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
      step done)

  (spp dynamic-a :reward 1)
  (spp dynamic-b :reward 3))
