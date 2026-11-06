(clear-all)

(define-model probe-imaginal-indirect
  (sgp :seed (100 0) :v nil :esc nil :rt -5 :ans nil
       :trace-detail high :imaginal-delay .2 :do-not-harvest imaginal)

  (chunk-type probe-goal state)
  (chunk-type result slot1 slot2)

  (add-dm
    (start isa probe-goal state 0)
    (r1 isa result slot1 red slot2 one)
    (r2 isa result slot1 blue slot2 two))

  (goal-focus start)

  (p make-imaginal
    =goal>
      isa probe-goal
      state 0
    ?imaginal>
      state free
    ==>
    +imaginal>
      isa result
      slot1 red
    =goal>
      state 1)

  (p modify-imaginal
    =goal>
      isa probe-goal
      state 1
    =imaginal>
      isa result
    ?imaginal>
      state free
    ==>
    =imaginal>
      slot2 one
    =goal>
      state 2)

  (p indirect-after-mod
    =goal>
      isa probe-goal
      state 2
    =imaginal>
      isa result
      slot1 red
    ?retrieval>
      state free
    ==>
    =imaginal>
      slot2 two
    +retrieval>
      =imaginal
    =goal>
      state 3)

  (p copy-retrieval
    =goal>
      isa probe-goal
      state 3
    =retrieval>
      isa result
    ?imaginal>
      state free
    ==>
    +imaginal>
      =retrieval
    =goal>
      state 4)

  (p clear-imaginal
    =goal>
      isa probe-goal
      state 4
    ?imaginal>
      state free
    ==>
    -imaginal>
    =goal>
      state 5))
