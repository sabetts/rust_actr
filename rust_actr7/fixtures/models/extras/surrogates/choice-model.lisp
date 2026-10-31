;; Project-authored choice/utility-learning surrogate for tests/surrogates.rs.
;; The task/reward structure follows the public ACT-R 6 unit-6 choice solution,
;; simplified to a goal-buffer task so the trace isolates utility learning.
(clear-all)
(define-model choice-surrogate
  (sgp :esc t :egs 0 :v nil :ul t :ult t)
  (chunk-type choose state item)
  (add-dm (goal isa choose state choose))
  (p heads
    =goal> isa choose state choose
    ?manual> state free
    ==>
    =goal> item heads state feedback
    +manual> cmd press-key key "h")
  (p tails
    =goal> isa choose state choose
    ?manual> state free
    ==>
    =goal> item tails state feedback
    +manual> cmd press-key key "t")
  (goal-focus goal)
  (spp heads :u 0.2)
  (spp tails :u 0)
)
