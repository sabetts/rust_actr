(clear-all)

(define-model probe-audio-trace
  (sgp :seed (100 0) :esc t :v t :trace-detail high :hear-self t
       :unstuff-aural-location .5)

  (chunk-type probe-goal state)
  (add-dm (start isa probe-goal state 0))
  (goal-focus start)

  (p speak-alpha
    =goal>
      isa probe-goal
      state 0
    ?vocal>
      state free
    ==>
    +vocal>
      isa speak
      string "alpha"
    =goal>
      state 1)

  (p speak-beta
    =goal>
      isa probe-goal
      state 1
    ?vocal>
      state free
    ==>
    +vocal>
      isa speak
      string "beta"
    =goal>
      state 2)
)
