(clear-all)

(define-model ul-test

(sgp :esc t :ul t :trace-detail high)

(chunk-type goal state)

(add-dm
 (s0 ISA goal state s0))

(goal-focus (ISA goal state s0))

;; R first so L (later) wins initial ties; a penalty then flips to R.
(p r
   =goal>
      ISA      goal
      state    s0
 ==>
   !output!    (R)
)

(p l
   =goal>
      ISA      goal
      state    s0
 ==>
   !output!    (L)
)
)
