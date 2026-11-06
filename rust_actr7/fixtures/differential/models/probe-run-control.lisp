(clear-all)

(define-model probe-run-control
  (sgp :seed (100 0) :esc t :v t :trace-detail high)

  (chunk-type probe-goal state)
  (add-dm (start isa probe-goal state 0))
  (goal-focus start)

  (p one
    =goal>
      isa probe-goal
      state 0
    ==>
    !output! "ONE"
    =goal>
      state 1)

  (p two
    =goal>
      isa probe-goal
      state 1
    ==>
    !output! "TWO"
    =goal>
      state 2)

  (p three
    =goal>
      isa probe-goal
      state 2
    ==>
    !output! "THREE"
    =goal>
      state 3)

  (p four
    =goal>
      isa probe-goal
      state 3
    ==>
    !output! "FOUR"
    =goal>
      state 4)

  (p five
    =goal>
      isa probe-goal
      state 4
    ==>
    !output! "FIVE"
    -goal>)
)
