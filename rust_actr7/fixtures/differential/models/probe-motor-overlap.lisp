(clear-all)

(let ((*standard-output* (make-broadcast-stream))
      (*error-output* (make-broadcast-stream)))
  (require-extra "extended-motor-actions"))

(define-model probe-motor-overlap
  (sgp :seed (100 0) :esc t :v t :trace-detail high
       :dual-processor-stages t :dual-execution-stages t
       :key-closure-time .01 :key-release-time .04)

  (chunk-type probe state)
  (add-dm (start isa probe state 0))
  (goal-focus start)

  (p hold-left
    =goal>
      isa probe
      state 0
    ?manual>
      state free
    ==>
    +manual>
      isa hold-key
      key "a"
    =goal>
      state 100)

  (p type-right
    =goal>
      isa probe
      state 1
    ?manual>
      preparation free
    ==>
    +manual>
      isa type-key
      key "l"
    =goal>
      state 100)

  (p release-left
    =goal>
      isa probe
      state 2
    ?manual>
      preparation free
    ==>
    +manual>
      isa release-key
      key "a"
    =goal>
      state 100))
