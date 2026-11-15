;;; Benchmark: declarative retrieval chain.
;;;
;;; Every production cycle performs one successful retrieval and requests the
;;; next fact in a five-chunk cycle.  Base-level learning and activation noise
;;; are disabled and `:le` is zero, so each retrieval has the same fixed
;;; latency; the measured work is request matching, activation lookup,
;;; retrieval selection, and buffer updates.
;;;
;;; The harness sends `clear-all` before loading this model.

(define-model bench-retrieval-loop
  (sgp :v nil :model-warnings nil :style-warnings nil
       :trace-detail low :esc t :seed (100 0)
       :bll nil :ans nil :blc 1 :rt -100 :lf 0.05 :le 0)

  (chunk-type goal state)
  (chunk-type fact number next)

  (add-dm
    (f1 isa fact number one next two)
    (f2 isa fact number two next three)
    (f3 isa fact number three next four)
    (f4 isa fact number four next five)
    (f5 isa fact number five next one))

  (define-chunks (s0 isa goal state s0))

  (goal-focus s0)

  (p start
    =goal>
      state s0
    ==>
    =goal>
      state count
    +retrieval>
      isa fact
      number one)

  (p next
    =goal>
      state count
    =retrieval>
      isa fact
      number =n
      next =m
    ==>
    +retrieval>
      isa fact
      number =m))
