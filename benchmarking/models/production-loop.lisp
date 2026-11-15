;;; Benchmark: pure production cycle.
;;;
;;; Two productions toggle the goal state forever.  There is no declarative
;;; memory and no device, so the measured work is production matching,
;;; conflict resolution, buffer writes, and scheduler bookkeeping.
;;;
;;; The harness sends `clear-all` before loading this model.  Run it directly
;;; with `actr run` (the Rust CLI) or from SBCL after loading ACT-R.

(define-model bench-production-loop
  (sgp :v nil :model-warnings nil :style-warnings nil
       :trace-detail low :esc t :seed (100 0))

  (chunk-type goal state)

  (define-chunks (s0 isa goal state s0))

  (goal-focus s0)

  (p a-to-b
    =goal>
      state s0
    ==>
    =goal>
      state s1)

  (p b-to-a
    =goal>
      state s1
    ==>
    =goal>
      state s0))
