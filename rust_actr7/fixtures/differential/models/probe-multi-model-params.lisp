(clear-all)

(define-model mp-a
  (sgp :seed (101 0) :esc t :v t :trace-detail high :ans .1 :rt -100)

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
    !output! "MP-A-DONE"
    =goal>
      state 2)

  (p failed
    =goal>
      isa probe-goal
      state 1
    ?retrieval>
      state error
    ==>
    !output! "MP-A-FAILED"
    =goal>
      state 2))

(define-model mp-b
  (sgp :seed (202 0) :esc t :v nil :trace-detail high :ans .5 :rt 100)

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
    !output! "MP-B-DONE"
    =goal>
      state 2)

  (p failed
    =goal>
      isa probe-goal
      state 1
    ?retrieval>
      state error
    ==>
    !output! "MP-B-FAILED"
    =goal>
      state 2))
