(clear-all)
(require-extra "blending")

(define-model probe-activation-combo
  (sgp :seed (100 0) :v t :trace-detail high :act high :pct t :esc t
       :bll .5 :mas 2 :ga 1 :ans .25 :rt 4 :mp 4 :blt t)

  (chunk-type target key value size)
  (chunk-type size (size-type t))
  (chunk-type probe-goal state focus)

  (define-chunks
    (key-1 isa chunk)
    (key-2 isa chunk))

  (add-dm
    (tiny isa size)
    (small isa size)
    (medium isa size)
    (large isa size)
    (x-large isa size)

    (a isa target key key-1 value 1 size large)
    (b isa target key key-1 value 2 size x-large)
    (c isa target key key-1 value 3 size tiny)
    (d isa target key key-2 value 1 size nil)
    (e isa target key key-2 value 3 size small)

    (goal isa probe-goal state 0 focus key-1))

  (goal-focus goal)

  (set-base-levels (a 4 -10) (b 4 -10) (c 5 -10) (d 2 -10) (e 1 -10))

  (set-similarities (tiny small -.1)
                    (small medium -.1)
                    (medium large -.1)
                    (large x-large -.1)
                    (tiny medium -.3)
                    (small large -.3)
                    (medium x-large -.3)
                    (tiny large -.6)
                    (small x-large -.6)
                    (tiny x-large -.9))

  (p retrieve
    =goal>
      isa probe-goal
      state 0
    ?retrieval>
      state free
    ==>
    +retrieval>
      isa target
      size medium
    =goal>
      state 1)

  (p blend
    =goal>
      isa probe-goal
      state 1
    ?blending>
      state free
      buffer empty
      error nil
    ==>
    +blending>
      isa target
    - key nil
    <= value 3
    > value 1
    =goal>
      state 2)

  (p report
    =goal>
      isa probe-goal
      state 2
    =blending>
      isa target
      value =val
      size =size
      key =key
    ==>
    !output! (ACTIVATION value =val size =size key =key)
    -goal>))
