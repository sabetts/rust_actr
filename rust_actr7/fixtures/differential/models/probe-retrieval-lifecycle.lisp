(clear-all)

(define-model probe-retrieval-lifecycle
  (sgp :seed (100 0) :esc t :v t :act t :trace-detail high
       :rt -5 :ans 0.01 :bll .5 :mas 2 :mp 0)

  (chunk-type item key val)
  (chunk-type probe-goal state)

  (add-dm (low-chunk isa item key 1 val 10)
          (high-chunk isa item key 2 val 20)
          (start isa probe-goal state 0))

  (set-base-levels (low-chunk 2) (high-chunk 5))

  (goal-focus start)

  (p request-low
    =goal>
      isa probe-goal
      state 0
    ==>
    +retrieval>
      isa item
      key 1
    =goal>
      state 1)

  (p abort-low
    =goal>
      isa probe-goal
      state 1
    ==>
    +retrieval>
      isa item
      key 2
    =goal>
      state 2)

  (p request-low-again
    =goal>
      isa probe-goal
      state 2
    =retrieval>
      isa item
      key 2
    ==>
    +retrieval>
      isa item
      key 1
      :recently-retrieved nil
    =goal>
      state 3)

  (p reject-recent-high
    =goal>
      isa probe-goal
      state 3
    =retrieval>
      isa item
      key 1
    ==>
    +retrieval>
      isa item
      key 2
      :recently-retrieved nil
    =goal>
      state 4)

  (p report-failure
    =goal>
      isa probe-goal
      state 4
    ?retrieval>
      state error
    ==>
    !output! "RETRIEVAL-FAILED"
    -goal>)
)
