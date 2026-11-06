(define-model mp-c
  (sgp :seed (303 0) :esc t :v t :trace-detail high :ans .3 :rt -200)

  (chunk-type probe-goal state)
  (chunk-type item value)
  (add-dm
    (start isa probe-goal state 0)
    (item-one isa item value "one")
    (item-two isa item value "two"))
  (goal-focus start)

  (p retrieve
    =goal>
      isa probe-goal
      state 0
    ?retrieval>
      state free
    ==>
    +retrieval>
      isa item
    =goal>
      state 1)

  (p found
    =goal>
      isa probe-goal
      state 1
    =retrieval>
      isa item
      value =v
    ==>
    !output! "MP-C-DONE"
    =goal>
      state 2)

  (p failed
    =goal>
      isa probe-goal
      state 1
    ?retrieval>
      state error
    ==>
    !output! "MP-C-FAILED"
    =goal>
      state 2))
