(clear-all)

(define-model instantiate-slot-name
    
  (sgp :epl t :pct t)
  
  (chunk-type goal target)
  
  (goal-focus (car (define-chunks (target color color red))))


 (p p1
    =goal>
      state nil
      target =feature
      =feature =t-value
    ==>
    =goal>
      state t
      target =feature
      =feature =t-value
    )

 (p p2
  !safe-eval! (not (equal =t-value 'blue))
  =goal>
    state t
    =feature =t-value
    target =feature
  ==>
  =goal>
    state 1))