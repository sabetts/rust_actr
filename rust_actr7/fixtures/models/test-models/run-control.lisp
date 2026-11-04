;; Purpose-built fixture for native Environment run-control tests.
;;
;; Two productions toggle the goal between FLIP and FLOP forever, so a run
;; keeps producing events and a break/stop request has time to arrive.

(clear-all)

(define-model run-control-fixture

  (sgp :esc t :lf .05 :v nil :trace-detail low)

  (chunk-type goal state)

  (goal-focus (ISA goal state flip))

  (p flip-a
     =goal>
       ISA goal
       state flip
   ==>
     =goal>
       state flop)

  (p flip-b
     =goal>
       ISA goal
       state flop
   ==>
     =goal>
       state flip)
)
