
(clear-all)

(define-model test
  (sgp :v t :trace-detail high)
  
  (install-device (open-exp-window ""))
  
  (start-hand-at-keypad)
  (chunk-type goal task step location)
  (goal-focus-fct (car (define-chunks (isa goal task punches) (punches) (pad) (home))))
  
  (p punches-1 
     =goal>
     task punches
     step nil
     ?manual>
     state free
     ==>
     +manual>
     isa punch
     hand right
     finger index
     *goal>
     step 2)
  
  (p punches-2
     =goal>
     task punches
     step 2
     ?manual>
     state free
     ==>
     +manual>
     isa punch
     hand right
     finger middle
     *goal>
       step 3)
  
  (p punches-3
     =goal>
     task punches
     step 3
     ?manual>
     state free
     ==>
     +manual>
     isa punch
     hand right
     finger ring
     *goal>
     step 4)
  
  
  (p punches-4
     =goal>
     task punches
     step 4
     ?manual>
     state free
     ==>
     +manual>
     isa punch
     hand right
     finger pinkie
     *goal>
     step 5)
  
  (p punches-5
     =goal>
     task punches
     step 5
     ?manual>
     state free
     ==>
     +manual>
     isa punch
     hand right
     finger thumb
     *goal>
     step 6)
  
  (p punches-6
     =goal>
     task punches
     step 6
     location nil
     ?manual>
     state free
     ==>
     +manual>
     isa hand-to-home
     *goal>
     step nil
     location home)
  
  (p punches-7
     =goal>
     task punches
     step 6
     location home
     ?manual>
     state free
     ==>
     +manual>
     isa hand-to-keypad
     *goal>
     step nil
     location pad)
  
  (p punches-8
     =goal>
     task punches
     step 6
     location pad
     ?manual>
     state free
     ==>
     *goal>
     task pad
     step nil
     location pad)

  
  (p pad-1
     =goal>
     task pad
     step nil
     ?manual>
     state free
     ==>
     +manual>
     isa press-key
     key clear
     *goal>
     step 2)
  
    (p pad-2
     =goal>
     task pad
     step 2
     ?manual>
     state free
     ==>
     +manual>
     isa press-key
     key keypad-=
     *goal>
       step 3)
  
  (p pad-3
     =goal>
     task pad
     step 3
     ?manual>
     state free
     ==>
     +manual>
     isa press-key
     key keypad-/
     *goal>
     step 4)
  
    (p pad-4
     =goal>
     task pad
     step 4
     ?manual>
     state free
     ==>
     +manual>
     isa press-key
     key keypad-*
     *goal>
       step 5)
  
    (p pad-5
     =goal>
     task pad
     step 5
     ?manual>
     state free
     ==>
     +manual>
     isa press-key
     key keypad-7
     *goal>
       step 6)
  
    (p pad-6
     =goal>
     task pad
     step 6
     ?manual>
     state free
     ==>
     +manual>
     isa press-key
     key keypad-8
     *goal>
       step 7)
  
    (p pad-7
     =goal>
     task pad
     step 7
     ?manual>
     state free
     ==>
     +manual>
     isa press-key
     key keypad-9
     *goal>
       step 8)
  
    (p pad-8
     =goal>
     task pad
     step 8
     ?manual>
     state free
     ==>
     +manual>
     isa press-key
     key keypad-minus
     *goal>
       step 9)
  
    (p pad-9
     =goal>
     task pad
     step 9
     ?manual>
     state free
     ==>
     +manual>
     isa press-key
     key keypad-4
     *goal>
       step 10)
  
    (p pad-10
     =goal>
     task pad
     step 10
     ?manual>
     state free
     ==>
     +manual>
     isa press-key
     key keypad-5
     *goal>
       step 11)
  
    (p pad-11
     =goal>
     task pad
     step 11
     ?manual>
     state free
     ==>
     +manual>
     isa press-key
     key keypad-6
     *goal>
       step 12)
  
    (p pad-12
     =goal>
     task pad
     step 12
     ?manual>
     state free
     ==>
     +manual>
     isa press-key
     key keypad-plus
     *goal>
       step 13)
  
    (p pad-13
     =goal>
     task pad
     step 13
     ?manual>
     state free
     ==>
     +manual>
     isa press-key
     key keypad-1
     *goal>
       step 14)
  
    (p pad-14
     =goal>
     task pad
     step 14
     ?manual>
     state free
     ==>
     +manual>
     isa press-key
     key keypad-2
     *goal>
       step 15)
  
    (p pad-15
     =goal>
     task pad
     step 15
     ?manual>
     state free
     ==>
     +manual>
     isa press-key
     key keypad-3
     *goal>
       step 16)
  
    (p pad-16
     =goal>
     task pad
     step 16
     ?manual>
     state free
     ==>
     +manual>
     isa press-key
     key keypad-0
     *goal>
       step 17)
  
    (p pad-17
     =goal>
     task pad
     step 17
     ?manual>
     state free
     ==>
     +manual>
     isa press-key
     key keypad-period
     *goal>
       step 18)
  
    (p pad-18
     =goal>
     task pad
     step 18
     ?manual>
     state free
     ==>
     +manual>
     isa press-key
     key enter
     *goal>
     step 19)
)