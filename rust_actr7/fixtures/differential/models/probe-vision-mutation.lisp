(clear-all)

(define-model probe-vision-mutation
  (sgp :seed (100 0) :esc t :v t :trace-detail high
       :unstuff-visual-location nil :visual-attention-latency 0)

  (chunk-type probe state)
  (add-dm (goal0 isa probe state 0))
  (goal-focus goal0)

  (p find-nearest-before
    =goal>
      isa probe
      state 1
    ?visual-location>
      state free
    ==>
    +visual-location>
      isa visual-location
      :nearest current
    =goal>
      state 2)

  (p report-nearest-before
    =goal>
      isa probe
      state 2
    =visual-location>
      isa visual-location
      screen-x =x
      screen-y =y
      value =v
    ==>
    !output! (NEAREST-BEFORE =visual-location =x =y =v)
    =goal>
      state 100)

  (p find-nearest-after-remove
    =goal>
      isa probe
      state 3
    ?visual-location>
      state free
    ==>
    +visual-location>
      isa visual-location
      :nearest current
    =goal>
      state 4)

  (p report-nearest-after-remove
    =goal>
      isa probe
      state 4
    =visual-location>
      isa visual-location
      screen-x =x
      screen-y =y
      value =v
    ==>
    !output! (NEAREST-AFTER-REMOVE =visual-location =x =y =v)
    =goal>
      state 100)

  (p find-two-feature
    =goal>
      isa probe
      state 5
    ?visual-location>
      state free
    ==>
    +visual-location>
      isa visual-location
      value "beta"
      color black
    =goal>
      state 6)

  (p report-two-feature
    =goal>
      isa probe
      state 6
    =visual-location>
      isa visual-location
      screen-x =x
      screen-y =y
      value =v
      color =c
    ==>
    !output! (TWO-FEATURE =visual-location =x =y =v =c)
    =goal>
      state 100)

  (p find-nearest-after-clear
    =goal>
      isa probe
      state 7
    ?visual-location>
      state free
    ==>
    +visual-location>
      isa visual-location
      :nearest current
    =goal>
      state 8)

  (p report-nearest-after-clear
    =goal>
      isa probe
      state 8
    =visual-location>
      isa visual-location
      screen-x =x
      screen-y =y
      value =v
    ==>
    !output! (NEAREST-AFTER-CLEAR =visual-location =x =y =v)
    =goal>
      state 100)

  (p find-nearest-same-time
    =goal>
      isa probe
      state 9
    ?visual-location>
      state free
    ==>
    +visual-location>
      isa visual-location
      :nearest current
    =goal>
      state 10)

  (p report-nearest-same-time
    =goal>
      isa probe
      state 10
    =visual-location>
      isa visual-location
      screen-x =x
      screen-y =y
      value =v
    ==>
    !output! (NEAREST-SAME-TIME =visual-location =x =y =v)
    =goal>
      state 100))
