(clear-all)

(define-model probe-runtime-defs
  (sgp :seed (100 0) :esc t :v t :trace-detail high)

  (chunk-type probe-goal state)
  (add-dm (start isa probe-goal state 0))
  (goal-focus start)

  (p first
    =goal>
      isa probe-goal
      state 0
    ==>
    !output! "FIRST"
    =goal>
      state 1)

  (p use-runtime
    =goal>
      isa probe-goal
      state 1
    ?retrieval>
      state free
    ==>
    !output! "USE-RUNTIME"
    +retrieval>
      isa probe-goal
      - state 0
      - state 1
    =goal>
      state 2)

  (p report-runtime
    =goal>
      isa probe-goal
      state 2
    =retrieval>
      isa probe-goal
      - state 0
      - state 1
    ==>
    !output! "REPORT-RUNTIME"
    -goal>)
)
