;; Purpose-built fixture for the Environment stepper RPC tests.
;;
;; The goal moves through a successful retrieval and then a retrieval
;; failure, so a stepping test sees conflict resolution, selection/firing,
;; start-retrieval, retrieved-chunk and retrieval-failure events without
;; depending on a tutorial model or an ACT-R checkout.

(clear-all)

(define-model stepper-fixture

  (sgp :esc t :lf .05 :trace-detail high :v t)

  (chunk-type goal state)
  (chunk-type item value)

  (add-dm
   (one ISA item value one))

  (goal-focus (ISA goal state start))

  (p begin
     =goal>
       ISA goal
       state start
   ==>
     =goal>
       state hit
     +retrieval>
       ISA item
       value one)

  (p got-hit
     =goal>
       ISA goal
       state hit
     =retrieval>
       ISA item
       value =value
   ==>
     =goal>
       state miss
     +retrieval>
       ISA item
       value missing)

  (p got-miss
     =goal>
       ISA goal
       state miss
     ?retrieval>
       state error
   ==>
     =goal>
       state done)
)
