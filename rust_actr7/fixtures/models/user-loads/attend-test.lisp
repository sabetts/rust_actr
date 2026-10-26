(clear-all)

(define-model attend

(sgp :esc t :trace-detail high)

(chunk-type goal step)

(add-dm
 (start isa chunk)
 (done isa chunk))

(p seek
   =goal>
      isa      goal
      step     start
 ==>
   +visual-location>
      isa      visual-location
   =goal>
      step     seeking
)

(p find-item
   =goal>
      isa      goal
      step     seeking
   =visual-location>
 ==>
   +visual>
      cmd      move-attention
      screen-pos =visual-location
   =goal>
      step     attending
)

(p read-and-respond
   =goal>
      isa      goal
      step     attending
   =visual>
      isa      visual-object
      value    =val
   ?manual>
      state    free
 ==>
   +manual>
      cmd      press-key
      key      =val
   =goal>
      step     done
)

(goal-focus (isa goal step start))
)
