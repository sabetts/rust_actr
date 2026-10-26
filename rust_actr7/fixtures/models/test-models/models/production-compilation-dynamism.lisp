(clear-all)

(define-model x
    (sgp :epl t :pct t :do-not-harvest imaginal)
  (sgp :trace-detail low)
  (chunk-type general slot1 slot2 slot3 slot4 test-num)
  
  (define-chunks (g test-num 1)
      (test slot1 color slot2 color slot3 value slot4 slot4)
    (slot1)
    (slot2)
    (slot3)
    (slot4)
    (value)
    (color))
  
  (declare-buffer-usage goal general :all)
  (declare-buffer-usage imaginal general :all)
  

  
  
  ;;;;;;;;;;;;;;;;;;;;;;;
  ;; simple case of a p1 RHS = & p2 LHS dynamic
  
  ;; both dynamic
  
  (p a1
     =goal>
     test-num 1
     value nil
     =imaginal>
     slot1 =s
     !safe-bind! =v 1
     ==>
     =imaginal>
     =goal>
     =s =v
     value t)
  
  (p b1
     =goal>
     test-num 1
     =s =y
     value t
     =imaginal>
     slot1 =s
     ==>
     =imaginal>
     !output! =y
     =goal>
     value busy)
  
  
  #|
   (p a1+b1
      =goal>
      test-num 1
      value nil
      =imaginal>
      slot1 =s
      !safe-bind! =v 1
      ==>
      =imaginal>
      !output! =v
      =goal>
      value busy
      =s =v)
      
  |#
  
  
  ;; only p1 RHS dynamic
  
    (p a2
     =goal>
     test-num 2
     value nil
     =imaginal>
     slot1 =s
     !safe-bind! =v 1
     ==>
     =imaginal>
     =goal>
     =s =v
     value t)
  
  (p b2
     =goal>
     test-num 2
     color =y
     value t
     =imaginal>
     slot1 =s
     ==>
     =imaginal>
     !output! =y
     =goal>
     value busy)
  
  
  #| (p a2+b2
       =goal>
         test-num 2
         value nil
       =imaginal>
          slot1 color
       !safe-bind! =v 1
      ==>
       =imaginal>
       !output! =v
       =goal>
       color =v
       value busy)
  |#
  
  ;; only p2 LHS dynamic
  
    (p a3
     =goal>
     test-num 3
     value nil
     =imaginal>
     slot1 =s
     !safe-bind! =v 1
     ==>
     =imaginal>
     =goal>
     color =v
     value t)
  
  (p b3
     =goal>
     test-num 3
     =s =y
     value t
     =imaginal>
     slot1 =s
     ==>
     =imaginal>
     !output! =y
     =goal>
     value busy)
  
  
  #| (p a3+b3
       =goal>
         test-num 3
         value nil
       =imaginal>
          slot1 color
       !safe-bind! =v 1
      ==>
       =imaginal>
       !output! =v
       =goal>
       color =v
       value busy)
  |#

  ;; both dynamic using different slot name binding location
  
    (p a4
     =goal>
     test-num 4
     value nil
     =imaginal>
     slot1 =s
     !safe-bind! =v 1
     ==>
     =imaginal>
     =goal>
     =s =v
     value t)
  
  (p b4
     =goal>
     test-num 4
     =s =y
     value t
     =imaginal>
     slot2 =s
     ==>
     =imaginal>
     !output! =y
     =goal>
     value busy)
  
  
    #| (p a4+b4
       =goal>
         test-num 4
         value nil
       =imaginal>
          slot1 =s
          slot2 =s
       !safe-bind! =v 1
      ==>
       =imaginal>
       !output! =v
       =goal>
       =s =v
       value busy)
  |#

  
  ;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;
  ;; Tests with 4 possible dynamic location
  ;; LHS,RHS = & LHS,RHS =
  
  ;; all 4 dynamic
  
  (p a5
     =goal>
     test-num 5
     value nil
     =s nil
     =imaginal>
     slot1 =s
     !safe-bind! =v 1
     ==>
     =imaginal>
     =goal>
     =s =v
     value t)
  
  (p b5
     =goal>
     test-num 5
     =s =y
     value t
     =imaginal>
     slot2 =s
     ==>
     =imaginal>
     !output! =y
     =goal>
     =s free
     value busy)
  
      #| (p a5+b5
       =goal>
         test-num 5
         value nil
         =s nil
       =imaginal>
          slot1 =s
          slot2 =s
       !safe-bind! =v 1
      ==>
       =imaginal>
       !output! =v
       =goal>
       =s free
       value busy)
  |#

  
  
  ;; p1 LHS static
  
    (p a6
     =goal>
     test-num 6
     value nil
     color nil
     =imaginal>
     slot1 =s
     !safe-bind! =v 1
     ==>
     =imaginal>
     =goal>
     =s =v
     value t)
  
  (p b6
     =goal>
     test-num 6
     =s =y
     value t
     =imaginal>
     slot2 =s
     ==>
     =imaginal>
     !output! =y
     =goal>
     =s free
     value busy)
  
  
  #| (p a6+b6
       =goal>
         test-num 6
         value nil
         color nil
       =imaginal>
          slot1 color
          slot2 color
       !safe-bind! =v 1
      ==>
       =imaginal>
       !output! =v
       =goal>
       color free
       value busy)
  |#

  ;; p1 RHS static
  
  (p a7
     =goal>
     test-num 7
     value nil
     =s nil
     =imaginal>
     slot1 =s
     !safe-bind! =v 1
     ==>
     =imaginal>
     =goal>
     color =v
     value t)
  
  (p b7
     =goal>
     test-num 7
     =s =y
     value t
     =imaginal>
     slot2 =s
     ==>
     =imaginal>
     !output! =y
     =goal>
     =s free
     value busy)
  
  
  #| (p a7+b7
       =goal>
         test-num 7
         value nil
         color nil
       =imaginal>
          slot1 color
          slot2 color
       !safe-bind! =v 1
      ==>
       =imaginal>
       !output! =v
       =goal>
       color free
       value busy)
  |#
  ;; p2 LHS static
  
   (p a8
     =goal>
     test-num 8
     value nil
     =s nil
     =imaginal>
     slot1 =s
     !safe-bind! =v 1
     ==>
     =imaginal>
     =goal>
     =s =v
     value t)
  
  (p b8
     =goal>
     test-num 8
     color =y
     value t
     =imaginal>
     slot2 =s
     ==>
     =imaginal>
     !output! =y
     =goal>
     =s free
     value busy) 
  
    #| (p a8+b8
       =goal>
         test-num 8
         value nil
         color nil
       =imaginal>
          slot1 color
          slot2 color
       !safe-bind! =v 1
      ==>
       =imaginal>
       !output! =v
       =goal>
       color free
       value busy)
  |#
  
  ;; p2 RHS static
  
    (p a9
     =goal>
     test-num 9
     value nil
     =s nil
     =imaginal>
     slot1 =s
     !safe-bind! =v 1
     ==>
     =imaginal>
     =goal>
     =s =v
     value t)
  
  (p b9
     =goal>
     test-num 9
     =s =y
     value t
     =imaginal>
     slot2 =s
     ==>
     =imaginal>
     !output! =y
     =goal>
     color free
     value busy)  
  
      #| (p a9+b9
       =goal>
         test-num 9
         value nil
         color nil
       =imaginal>
          slot1 color
          slot2 color
       !safe-bind! =v 1
      ==>
       =imaginal>
       !output! =v
       =goal>
       color free
       value busy)
  |#
  
  ;; p1 LHS and RHS static
  
  (p a10
     =goal>
     test-num 10
     value nil
     color nil
     =imaginal>
     slot1 =s
     !safe-bind! =v 1
     ==>
     =imaginal>
     =goal>
     color =v
     value t)
  
  (p b10
     =goal>
     test-num 10
     =s =y
     value t
     =imaginal>
     slot2 =s
     ==>
     =imaginal>
     !output! =y
     =goal>
     =s free
     value busy)  
  
        #| (p a10+b10
       =goal>
         test-num 10
         value nil
         color nil
       =imaginal>
          slot1 =s
          slot2 color
       !safe-bind! =v 1
      ==>
       =imaginal>
       !output! =v
       =goal>
       color free
       value busy)
  |#

  
  
  
  ;; p1 lhs and p2 lhs static
  
  (p a11
     =goal>
     test-num 11
     value nil
     color nil
     =imaginal>
     slot1 =s
     !safe-bind! =v 1
     ==>
     =imaginal>
     =goal>
     =s =v
     value t)
  
  (p b11
     =goal>
     test-num 11
     color =y
     value t
     =imaginal>
     slot2 =s
     ==>
     =imaginal>
     !output! =y
     =goal>
     =s free
     value busy)  
  
  #| (p a11+b11
       =goal>
         test-num 11
         value nil
         color nil
       =imaginal>
          slot1 color
          slot2 color
       !safe-bind! =v 1
      ==>
       =imaginal>
       !output! =v
       =goal>
       color free
       value busy)
  |#
 
  ;; p1 lhs and p2 rhs static
  
    (p a12
     =goal>
     test-num 12
     value nil
     color nil
     =imaginal>
     slot1 =s
     !safe-bind! =v 1
     ==>
     =imaginal>
     =goal>
     =s =v
     value t)
  
  (p b12
     =goal>
     test-num 12
     =s =y
     value t
     =imaginal>
     slot2 =s
     ==>
     =imaginal>
     !output! =y
     =goal>
     color free
     value busy)  
  #|
(p a12+b12
   =goal>
   test-num 12 
   value nil
   color nil
   =imaginal>
   slot1 color
   slot2 color
   !safe-bind! =v 1
   ==>
   =imaginal>
   !output! =v
   =goal>
   color free
   value busy)
|#

  
  ;; P1 rhs and p2 lhs static
  
      (p a13
     =goal>
     test-num 13
     value nil
     =s nil
     =imaginal>
     slot1 =s
     !safe-bind! =v 1
     ==>
     =imaginal>
     =goal>
     color =v
     value t)
  
  (p b13
     =goal>
     test-num 13
     color =y
     value t
     =imaginal>
     slot2 =s
     ==>
     =imaginal>
     !output! =y
     =goal>
     =s free
     value busy)  


#|

(p a13+b13
   =goal>
   test-num 13
   value nil
   color nil
   =imaginal>
   slot1 color
   slot2 color
   
   !safe-bind! =v 1
   ==>
   =imaginal>
   
   !output! =v
   
   =goal>
   color free
   value busy
   )
|#
  
  ;; p1 rhs and p2 rhs static
  
  (p a14
     =goal>
     test-num 14
     value nil
     =s nil
     =imaginal>
     slot1 =s
     !safe-bind! =v 1
     ==>
     =imaginal>
     =goal>
     color =v
     value t)
  
  (p b14
     =goal>
     test-num 14
     =s =y
     value t
     =imaginal>
     slot2 =s
     ==>
     =imaginal>
     !output! =y
     =goal>
     color free
     value busy)
  
  
#|

(p a14+b14
   =goal>
   test-num 14
   value nil
   color nil
   
   =imaginal>
   slot1 color
   slot2 color
   
   !safe-bind! =v 1
   ==>
   =imaginal>
   
   !output! =v
   
   =goal>
   color free
   value busy
   )
|#
    ;; p2 lhs and rhs static
  
  (p a15
     =goal>
     test-num 15
     value nil
     =s nil
     =imaginal>
     slot1 =s
     !safe-bind! =v 1
     ==>
     =imaginal>
     =goal>
     =s =v
     value t)
  
  (p b15
     =goal>
     test-num 15
     color =y
     value t
     =imaginal>
     slot2 =s
     ==>
     =imaginal>
     !output! =y
     =goal>
     color free
     value busy)
 
#|

(p a15+b15
   =goal>
   test-num 15
   value nil
   color     nil
   
   =imaginal>
   slot1 color
   slot2 =s
   
   !safe-bind! =v 1
   ==>
   =imaginal>
   
   !output! =v
   
   =goal>
   color      free
   value busy
   )
|# 
  
  ;; only p1 lhs dynamic
  
    (p a16
     =goal>
     test-num 16
     value nil
     =s nil
     =imaginal>
     slot1 =s
     !safe-bind! =v 1
     ==>
     =imaginal>
     =goal>
     color =v
     value t)
  
  (p b16
     =goal>
     test-num 16
     color =y
     value t
     =imaginal>
     slot2 =s
     ==>
     =imaginal>
     !output! =y
     =goal>
     color free
     value busy)
  
 #|

(p a16+b16
   =goal>
   test-num 16
   value nil
   color     nil
   
   =imaginal>
   slot1 color
   slot2 =s2
   
   !safe-bind! =v 1
   ==>
   =imaginal>
   
   !output! =v
   
   =goal>
   color      free
   value busy
   )
|# 
  ;; only p1 rhs dynamic
  
  (p a17
     =goal>
     test-num 17
     value nil
     color nil
     =imaginal>
     slot1 =s
     !safe-bind! =v 1
     ==>
     =imaginal>
     =goal>
     =s =v
     value t)
  
  (p b17
     =goal>
     test-num 17
     color =y
     value t
     =imaginal>
     slot2 =s
     ==>
     =imaginal>
     !output! =y
     =goal>
     color free
     value busy)
 
#|

(p a17+b17
   =goal>
   test-num 17
   value nil
   color     nil
   
   =imaginal>
   slot1 color
   slot2 =s
   
   !safe-bind! =v 1
   ==>
   =imaginal>
   
   !output! =v
   
   =goal>
   color      free
   value busy
   )
|# 
    ;; only p2 lhs dynamic
  
  (p a18
     =goal>
     test-num 18
     value nil
     color nil
     =imaginal>
     slot1 =s
     !safe-bind! =v 1
     ==>
     =imaginal>
     =goal>
     color =v
     value t)
  
  (p b18
     =goal>
     test-num 18
     =s =y
     value t
     =imaginal>
     slot2 =s
     ==>
     =imaginal>
     !output! =y
     =goal>
     color free
     value busy)
 
#|

(p a18+b18
   =goal>
   test-num 18
   value nil
   color     nil
   
   =imaginal>
   slot1 =s 
   slot2 color
   
   !safe-bind! =v 1
   ==>
   =imaginal>
   
   !output! =v
   
   =goal>
   color      free
   value busy
   )
|# 
    ;; only p2 rhs dynamic
  
  (p a19
     =goal>
     test-num 19
     value nil
     color nil
     =imaginal>
     slot1 =s
     !safe-bind! =v 1
     ==>
     =imaginal>
     =goal>
     color =v
     value t)
  
  (p b19
     =goal>
     test-num 19
     color =y
     value t
     =imaginal>
     slot2 =s
     ==>
     =imaginal>
     !output! =y
     =goal>
     =s free
     value busy)

#|

give this one a close look because if the action is variablized it needs to also have color =v

(p a19+b19
   =goal>
   test-num 19
   value nil
   color     nil
   
   =imaginal>
   slot1 =s
   slot2 color
   
   !safe-bind! =v 1
   ==>
   =imaginal>
   
   !output! =v
   
   =goal>
   color      free
   value busy
   )
|#  
  
  ;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;
  ;; Tests with 4 possible dynamic location
  ;; LHS,RHS + & LHS,RHS =  
  
  ;; all 4 dynamic
  
  (p a20
     =goal>
     test-num 20
     value nil
     =s nil
     =imaginal>
     slot1 =s
     !safe-bind! =v 1
     ==>
     =imaginal>
     +goal>
     test-num 20
     =s =v
     value t)
  
  (p b20
     =goal>
     test-num 20
     =s =y
     value t
     =imaginal>
     slot2 =s
     ==>
     =imaginal>
     !output! =y
     =goal>
     =s free
     value busy)
  

#|

(p a20+b20
   =goal>
   test-num 20
   value nil
   =s     nil
   
   =imaginal>
   slot1 =s
   slot2 =s
   
   !safe-bind! =v 1
   ==>
   =imaginal>
   
   !output! =v
   
   +goal>
   test-num 20
   =s      free
   value busy
   )
|#

  ;; p1 LHS static
  
    (p a21
     =goal>
     test-num 21
     value nil
     color nil
     =imaginal>
     slot1 =s
     !safe-bind! =v 1
     ==>
     =imaginal>
     +goal>
     test-num 21
     =s =v
     value t)
  
  (p b21
     =goal>
     test-num 21
     =s =y
     value t
     =imaginal>
     slot2 =s
     ==>
     =imaginal>
     !output! =y
     =goal>
     =s free
     value busy)
  

#|

(p a21+b21
   =goal>
   test-num 21
   value nil
   color     nil
   
   =imaginal>
   slot1 =s 
   slot2 =s
   
   !safe-bind! =v 1
   ==>
   =imaginal>
   
   !output! =v
   
   +goal>
   test-num 21
   =s      free
   value busy
   )
|#


  ;; p1 RHS static
  
  (p a22
     =goal>
     test-num 22
     value nil
     =s nil
     =imaginal>
     slot1 =s
     !safe-bind! =v 1
     ==>
     =imaginal>
     +goal>
     test-num 22
     color =v
     value t)
  
  (p b22
     =goal>
     test-num 22
     =s =y
     value t
     =imaginal>
     slot2 =s
     ==>
     =imaginal>
     !output! =y
     =goal>
     =s free
     value busy)
#|

(p a22+b22
   =goal>
   test-num 22
   value nil
   =s     nil
   
   =imaginal>
   slot1 =s
   slot2 color
   
   !safe-bind! =v 1
   ==>
   =imaginal>
   
   !output! =v
   
   +goal>
   test-num 22
   color      free
   value busy
   )
|#  
  ;; p2 LHS static
  
   (p a23
     =goal>
     test-num 23
     value nil
     =s nil
     =imaginal>
     slot1 =s
     !safe-bind! =v 1
     ==>
     =imaginal>
     +goal>
     test-num 23
     =s =v
     value t)
  
  (p b23
     =goal>
     test-num 23
     color =y
     value t
     =imaginal>
     slot2 =s
     ==>
     =imaginal>
     !output! =y
     =goal>
     =s free
     value busy) 
  #|

(p a23+b23
   =goal>
   test-num 23
   value nil
   color     nil
   
   =imaginal>
   slot1 color
   slot2 color
   
   !safe-bind! =v 1
   ==>
   =imaginal>
   
   !output! =v
   
   +goal>
   test-num 23
   color      free
   value busy
   )
|#
  ;; p2 RHS static
  
    (p a24
     =goal>
     test-num 24
     value nil
     =s nil
     =imaginal>
     slot1 =s
     !safe-bind! =v 1
     ==>
     =imaginal>
     +goal>
     test-num 24
     =s =v
     value t)
  
  (p b24
     =goal>
     test-num 24
     =s =y
     value t
     =imaginal>
     slot2 =s
     ==>
     =imaginal>
     !output! =y
     =goal>
     color free
     value busy)  
  
#|

(p a24+b24
   =goal>
   test-num 24
   value nil
   color     nil
   
   =imaginal>
   slot1 color
   slot2 color
   
   !safe-bind! =v 1
   ==>
   =imaginal>
   
   !output! =v
   
   +goal>
   test-num 24
   color      free
   value busy
   )
|#
  
  ;; p1 LHS and RHS static
  
  (p a25
     =goal>
     test-num 25
     value nil
     color nil
     =imaginal>
     slot1 =s
     !safe-bind! =v 1
     ==>
     =imaginal>
     +goal>
     test-num 25
     color =v
     value t)
  
  (p b25
     =goal>
     test-num 25
     =s =y
     value t
     =imaginal>
     slot2 =s
     ==>
     =imaginal>
     !output! =y
     =goal>
     =s free
     value busy)  
  
#|

(p a25+b25
   =goal>
   test-num 25
   value nil
   color     nil
   
   =imaginal>
   slot1 =s
   slot2 color
   
   !safe-bind! =v 1
   ==>
   =imaginal>
   
   !output! =v
   
   +goal>
   test-num 25
   color      free
   value busy
   )
|#
  ;; p1 lhs and p2 lhs static
  
  (p a26
     =goal>
     test-num 26
     value nil
     color nil
     =imaginal>
     slot1 =s
     !safe-bind! =v 1
     ==>
     =imaginal>
     +goal>
      test-num 26
    =s =v
     value t)
  
  (p b26
     =goal>
     test-num 26
     color =y
     value t
     =imaginal>
     slot2 =s
     ==>
     =imaginal>
     !output! =y
     =goal>
     =s free
     value busy)  
  
  #|

(p a26+b26
   =goal>
   test-num 26
   value nil
   color     nil
   
   =imaginal>
   slot1 color 
   slot2 color
   
   !safe-bind! =v 1
   ==>
   =imaginal>
   
   !output! =v
   
   +goal>
   test-num 26
   color      free
   value busy
   )
|#

  ;; p1 lhs and p2 rhs static
  
    (p a27
     =goal>
     test-num 27
     value nil
     color nil
     =imaginal>
     slot1 =s
     !safe-bind! =v 1
     ==>
     =imaginal>
     +goal>
     test-num 27
     =s =v
     value t)
  
  (p b27
     =goal>
     test-num 27
     =s =y
     value t
     =imaginal>
     slot2 =s
     ==>
     =imaginal>
     !output! =y
     =goal>
     color free
     value busy)  
  
  #|

(p a27+b27
   =goal>
   test-num 27
   value nil
   color     nil
   
   =imaginal>
   slot1 color 
   slot2 color
   
   !safe-bind! =v 1
   ==>
   =imaginal>
   
   !output! =v
   
   +goal>
   test-num 27
   color      free
   value busy
   )
|#
  ;; P1 rhs and p2 lhs static
  
      (p a28
     =goal>
     test-num 28
     value nil
     =s nil
     =imaginal>
     slot1 =s
     !safe-bind! =v 1
     ==>
     =imaginal>
     +goal>
     test-num 28
     color =v
     value t)
  
  (p b28
     =goal>
     test-num 28
     color =y
     value t
     =imaginal>
     slot2 =s
     ==>
     =imaginal>
     !output! =y
     =goal>
     =s free
     value busy)  
  
#|

(p a28+b28
   =goal>
   test-num 28
   value nil
    =s    nil
   
   =imaginal>
   slot1 =s
   slot2 color
   
   !safe-bind! =v 1
   ==>
   =imaginal>
   
   !output! =v
   
   +goal>
   test-num 28
   color      free
   value busy
   )
|#
  ;; p1 rhs and p2 rhs static
  
  (p a29
     =goal>
     test-num 29
     value nil
     =s nil
     =imaginal>
     slot1 =s
     !safe-bind! =v 1
     ==>
     =imaginal>
     +goal>
     test-num 29
     color =v
     value t)
  
  (p b29
     =goal>
     test-num 29
     =s =y
     value t
     =imaginal>
     slot2 =s
     ==>
     =imaginal>
     !output! =y
     =goal>
     color free
     value busy)
  

#|

(p a29+b29
   =goal>
   test-num 29
   value nil
   =s     nil
   
   =imaginal>
   slot1 =s
   slot2 color
   
   !safe-bind! =v 1
   ==>
   =imaginal>
   
   !output! =v
   
   +goal>
   test-num 29
    color     free
   value busy
   )
|#
  
    ;; p2 lhs and rhs static
  
  (p a30
     =goal>
     test-num 30
     value nil
     =s nil
     =imaginal>
     slot1 =s
     !safe-bind! =v 1
     ==>
     =imaginal>
     +goal>
     test-num 30
     =s =v
     value t)
  
  (p b30
     =goal>
     test-num 30
     color =y
     value t
     =imaginal>
     slot2 =s
     ==>
     =imaginal>
     !output! =y
     =goal>
     color free
     value busy)
#|

(p a30+b30
   =goal>
   test-num 30
   value nil
    color    nil
   
   =imaginal>
   slot1 color
   slot2 =s
   
   !safe-bind! =v 1
   ==>
   =imaginal>
   
   !output! =v
   
   +goal>
   test-num 30
   color      free
   value busy
   )
|#  
  
  ;; only p1 lhs dynamic
  
    (p a31
     =goal>
     test-num 31
     value nil
     =s nil
     =imaginal>
     slot1 =s
     !safe-bind! =v 1
     ==>
     =imaginal>
     +goal>
     test-num 31
     color =v
     value t)
  
  (p b31
     =goal>
     test-num 31
     color =y
     value t
     =imaginal>
     slot2 =s
     ==>
     =imaginal>
     !output! =y
     =goal>
     color free
     value busy)
  
  
#|

(p a31+b31
   =goal>
   test-num 31
   value nil
   =s     nil
   
   =imaginal>
   slot1 =s
   slot2 =s1
   
   !safe-bind! =v 1
   ==>
   =imaginal>
   
   !output! =v
   
   +goal>
   test-num 31
   color      free
   value busy
   )
|#
  ;; only p1 rhs dynamic
  
  (p a32
     =goal>
     test-num 32
     value nil
     color nil
     =imaginal>
     slot1 =s
     !safe-bind! =v 1
     ==>
     =imaginal>
     +goal>
     test-num 32
     =s =v
     value t)
  
  (p b32
     =goal>
     test-num 32
     color =y
     value t
     =imaginal>
     slot2 =s
     ==>
     =imaginal>
     !output! =y
     =goal>
     color free
     value busy)
  
#|

(p a32+b32
   =goal>
   test-num 32
   value nil
   color     nil
   
   =imaginal>
   slot1 color 
   slot2 =s
   
   !safe-bind! =v 1
   ==>
   =imaginal>
   
   !output! =v
   
   +goal>
   test-num 32
   color      free
   value busy
   )
|#

    ;; only p2 lhs dynamic
  
  (p a33
     =goal>
     test-num 33
     value nil
     color nil
     =imaginal>
     slot1 =s
     !safe-bind! =v 1
     ==>
     =imaginal>
     +goal>
     test-num 33
     color =v
     value t)
  
  (p b33
     =goal>
     test-num 33
     =s =y
     value t
     =imaginal>
     slot2 =s
     ==>
     =imaginal>
     !output! =y
     =goal>
     color free
     value busy)
  #|

(p a33+b33
   =goal>
   test-num 33
   value nil
   color     nil
   
   =imaginal>
   slot1 =s
   slot2 color
   
   !safe-bind! =v 1
   ==>
   =imaginal>
   
   !output! =v
   
   +goal>
   test-num 33
    color     free
   value busy
   )
|#
    ;; only p2 rhs dynamic
  
  (p a34
     =goal>
     test-num 34
     value nil
     color nil
     =imaginal>
     slot1 =s
     !safe-bind! =v 1
     ==>
     =imaginal>
     +goal>
      test-num 34
    color =v
     value t)
  
  (p b34
     =goal>
     test-num 34
     color =y
     value t
     =imaginal>
     slot2 =s
     ==>
     =imaginal>
     !output! =y
     =goal>
     =s free
     value busy)
  
  #|

(p a34+b34
   =goal>
   test-num 34
   value nil
   color     nil
   
   =imaginal>
   slot1 =s
   slot2 color
   
   !safe-bind! =v 1
   ==>
   =imaginal>
   
   !output! =v
   
   +goal>
   test-num 34
   color      free
   value busy
   )
|#
  ;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;
  ;; Tests with 4 possible dynamic location
  ;; LHS,RHS * & LHS,RHS *  
  
  ;; all 4 dynamic
  
  (p a35
     =goal>
     test-num 35
     value nil
     =s nil
     =imaginal>
     slot1 =s
     !safe-bind! =v 1
     ==>
     =imaginal>
     *goal>
     =s =v
     value t)
  
  (p b35
     =goal>
     test-num 35
     =s =y
     value t
     =imaginal>
     slot2 =s
     ==>
     =imaginal>
     !output! =y
     *goal>
     =s free
     value busy)
 #|

(p a35+b35
   =goal>
   test-num 35
   value nil
    =s    nil
   
   =imaginal>
   slot1 =s
   slot2 =s
   
   !safe-bind! =v 1
   ==>
   =imaginal>
   
   !output! =v
   
   *goal>
    =s     free
   value busy
   )
|# 
  ;; p1 LHS static
  
    (p a36
     =goal>
     test-num 36
     value nil
     color nil
     =imaginal>
     slot1 =s
     !safe-bind! =v 1
     ==>
     =imaginal>
     *goal>
     =s =v
     value t)
  
  (p b36
     =goal>
     test-num 36
     =s =y
     value t
     =imaginal>
     slot2 =s
     ==>
     =imaginal>
     !output! =y
     *goal>
     =s free
     value busy)
 #|

(p a36+b36
   =goal>
   test-num 36
   value nil
   color     nil
   
   =imaginal>
   slot1 color
   slot2 color
   
   !safe-bind! =v 1
   ==>
   =imaginal>
   
   !output! =v
   
   *goal>
     color    free
   value busy
   )
|# 
  ;; p1 RHS static
  
  (p a37
     =goal>
     test-num 37
     value nil
     =s nil
     =imaginal>
     slot1 =s
     !safe-bind! =v 1
     ==>
     =imaginal>
     *goal>
     color =v
     value t)
  
  (p b37
     =goal>
     test-num 37
     =s =y
     value t
     =imaginal>
     slot2 =s
     ==>
     =imaginal>
     !output! =y
     *goal>
     =s free
     value busy)
  #|

(p a37+b37
   =goal>
   test-num 37
   value nil
    color    nil
   
   =imaginal>
   slot1 color
   slot2 color
   
   !safe-bind! =v 1
   ==>
   =imaginal>
   
   !output! =v
   
   *goal>
    color     free
   value busy
   )
|#  
  ;; p2 LHS static
  
   (p a38
     =goal>
     test-num 38
     value nil
     =s nil
     =imaginal>
     slot1 =s
     !safe-bind! =v 1
     ==>
     =imaginal>
     *goal>
     =s =v
     value t)
  
  (p b38
     =goal>
     test-num 38
     color =y
     value t
     =imaginal>
     slot2 =s
     ==>
     =imaginal>
     !output! =y
     *goal>
     =s free
     value busy) 
 
 #|

(p a38+b38
   =goal>
   test-num 38
   value nil
    color    nil
   
   =imaginal>
   slot1 color
   slot2 color
   
   !safe-bind! =v 1
   ==>
   =imaginal>
   
   !output! =v
   
   *goal>
    color     free
   value busy
   )
|#  
  ;; p2 RHS static
  
    (p a39
     =goal>
     test-num 39
     value nil
     =s nil
     =imaginal>
     slot1 =s
     !safe-bind! =v 1
     ==>
     =imaginal>
     *goal>
     =s =v
     value t)
  
  (p b39
     =goal>
     test-num 39
     =s =y
     value t
     =imaginal>
     slot2 =s
     ==>
     =imaginal>
     !output! =y
     *goal>
     color free
     value busy)  
  
   #|

(p a39+b39
   =goal>
   test-num 39
   value nil
   color     nil
   
   =imaginal>
   slot1 color
   slot2 color
   
   !safe-bind! =v 1
   ==>
   =imaginal>
   
   !output! =v
   
   *goal>
    color     free
   value busy
   )
|# 

  ;; p1 LHS and RHS static
  
  (p a40
     =goal>
     test-num 40
     value nil
     color nil
     =imaginal>
     slot1 =s
     !safe-bind! =v 1
     ==>
     =imaginal>
     *goal>
     color =v
     value t)
  
  (p b40
     =goal>
     test-num 40
     =s =y
     value t
     =imaginal>
     slot2 =s
     ==>
     =imaginal>
     !output! =y
     *goal>
     =s free
     value busy)  
  
 #|

(p a40+b40
   =goal>
   test-num 40
   value nil
    color    nil
   
   =imaginal>
   slot1 =s
   slot2 color
   
   !safe-bind! =v 1
   ==>
   =imaginal>
   
   !output! =v
   
   *goal>
   color      free
   value busy
   )
|# 
  ;; p1 lhs and p2 lhs static
  
  (p a41
     =goal>
     test-num 41
     value nil
     color nil
     =imaginal>
     slot1 =s
     !safe-bind! =v 1
     ==>
     =imaginal>
     *goal>
     =s =v
     value t)
  
  (p b41
     =goal>
     test-num 41
     color =y
     value t
     =imaginal>
     slot2 =s
     ==>
     =imaginal>
     !output! =y
     *goal>
     =s free
     value busy)  
  
   #|

(p a41+b41
   =goal>
   test-num 41
   value nil
    color    nil
   
   =imaginal>
   slot1 color
   slot2 color
   
   !safe-bind! =v 1
   ==>
   =imaginal>
   
   !output! =v
   
   *goal>
    color     free
   value busy
   )
|# 

  ;; p1 lhs and p2 rhs static
  
    (p a42
     =goal>
     test-num 42
     value nil
     color nil
     =imaginal>
     slot1 =s
     !safe-bind! =v 1
     ==>
     =imaginal>
     *goal>
     =s =v
     value t)
  
  (p b42
     =goal>
     test-num 42
     =s =y
     value t
     =imaginal>
     slot2 =s
     ==>
     =imaginal>
     !output! =y
     *goal>
     color free
     value busy)  
  
   #|

(p a42+b42
   =goal>
   test-num 42
   value nil
   color     nil
   
   =imaginal>
   slot1 color
   slot2 color
   
   !safe-bind! =v 1
   ==>
   =imaginal>
   
   !output! =v
   
   *goal>
   color      free
   value busy
   )
|# 
  ;; P1 rhs and p2 lhs static
  
      (p a43
     =goal>
     test-num 43
     value nil
     =s nil
     =imaginal>
     slot1 =s
     !safe-bind! =v 1
     ==>
     =imaginal>
     *goal>
     color =v
     value t)
  
  (p b43
     =goal>
     test-num 43
     color =y
     value t
     =imaginal>
     slot2 =s
     ==>
     =imaginal>
     !output! =y
     *goal>
     =s free
     value busy)  
  
   #|

(p a43+b43
   =goal>
   test-num 43
   value nil
   color     nil
   
   =imaginal>
   slot1 color
   slot2 color
   
   !safe-bind! =v 1
   ==>
   =imaginal>
   
   !output! =v
   
   *goal>
   color      free
   value busy
   )
|# 


  ;; p1 rhs and p2 rhs static
  
  (p a44
     =goal>
     test-num 44
     value nil
     =s nil
     =imaginal>
     slot1 =s
     !safe-bind! =v 1
     ==>
     =imaginal>
     *goal>
     color =v
     value t)
  
  (p b44
     =goal>
     test-num 44
     =s =y
     value t
     =imaginal>
     slot2 =s
     ==>
     =imaginal>
     !output! =y
     *goal>
     color free
     value busy)
  
     #|

(p a44+b44
   =goal>
   test-num 44
   value nil
    color    nil
   
   =imaginal>
   slot1 color
   slot2 color
   
   !safe-bind! =v 1
   ==>
   =imaginal>
   
   !output! =v
   
   *goal>
   color      free
   value busy
   )
|# 
    ;; p2 lhs and rhs static
  
  (p a45
     =goal>
     test-num 45
     value nil
     =s nil
     =imaginal>
     slot1 =s
     !safe-bind! =v 1
     ==>
     =imaginal>
     *goal>
     =s =v
     value t)
  
  (p b45
     =goal>
     test-num 45
     color =y
     value t
     =imaginal>
     slot2 =s
     ==>
     =imaginal>
     !output! =y
     *goal>
     color free
     value busy)
  
     #|

(p a45+b45
   =goal>
   test-num 45
   value nil
   color     nil
   
   =imaginal>
   slot1 color
   slot2 =s
   
   !safe-bind! =v 1
   ==>
   =imaginal>
   
   !output! =v
   
   *goal>
    color     free
   value busy
   )
|# 
  ;; only p1 lhs dynamic
  
    (p a46
     =goal>
     test-num 46
     value nil
     =s nil
     =imaginal>
     slot1 =s
     !safe-bind! =v 1
     ==>
     =imaginal>
     *goal>
     color =v
     value t)
  
  (p b46
     =goal>
     test-num 46
     color =y
     value t
     =imaginal>
     slot2 =s
     ==>
     =imaginal>
     !output! =y
     *goal>
     color free
     value busy)
  
     #|

(p a46+b46
   =goal>
   test-num 46
   value nil
     =s   nil
   
   =imaginal>
   slot1 =s
   slot2 =s1
   
   !safe-bind! =v 1
   ==>
   =imaginal>
   
   !output! =v
   
   *goal>
   color      free
   value busy
   )
|# 

  ;; only p1 rhs dynamic
  
  (p a47
     =goal>
     test-num 47
     value nil
     color nil
     =imaginal>
     slot1 =s
     !safe-bind! =v 1
     ==>
     =imaginal>
     *goal>
     =s =v
     value t)
  
  (p b47
     =goal>
     test-num 47
     color =y
     value t
     =imaginal>
     slot2 =s
     ==>
     =imaginal>
     !output! =y
     *goal>
     color free
     value busy)
     #|

(p a47+b47
   =goal>
   test-num 47
   value nil
   color     nil
   
   =imaginal>
   slot1 color 
   slot2 =s
   
   !safe-bind! =v 1
   ==>
   =imaginal>
   
   !output! =v
   
   *goal>
   color      free
   value busy
   )
|# 
    ;; only p2 lhs dynamic
  
  (p a48
     =goal>
     test-num 48
     value nil
     color nil
     =imaginal>
     slot1 =s
     !safe-bind! =v 1
     ==>
     =imaginal>
     *goal>
     color =v
     value t)
  
  (p b48
     =goal>
     test-num 48
     =s =y
     value t
     =imaginal>
     slot2 =s
     ==>
     =imaginal>
     !output! =y
     *goal>
     color free
     value busy)
     #|

(p a48+b48
   =goal>
   test-num 48
   value nil
   color     nil
   
   =imaginal>
   slot1 =s
   slot2 color
   
   !safe-bind! =v 1
   ==>
   =imaginal>
   
   !output! =v
   
   *goal>
   color      free
   value busy
   )
|# 
    ;; only p2 rhs dynamic
  
  (p a49
     =goal>
     test-num 49
     value nil
     color nil
     =imaginal>
     slot1 =s
     !safe-bind! =v 1
     ==>
     =imaginal>
     *goal>
     color =v
     value t)
  
  (p b49
     =goal>
     test-num 49
     color =y
     value t
     =imaginal>
     slot2 =s
     ==>
     =imaginal>
     !output! =y
     *goal>
     =s free
     value busy)
  
     #|

(p a49+b49
   =goal>
   test-num 49
   value nil
   color     nil
   
   =imaginal>
   slot1 =s 
   slot2 color
   
   !safe-bind! =v 1
   ==>
   =imaginal>
   
   !output! =v
   
   *goal>
   color  free
   value busy
   )
|# 
   ;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;
  ;; Tests with 5 possible dynamic location
  ;; LHS,RHS *,+ & LHS,RHS *  
  
  ;; all 5 dynamic
  
  (p a50
     =goal>
     test-num 50
     value nil
     =s nil
     =imaginal>
     slot1 =s
     !safe-bind! =v 1
     ==>
     =imaginal>
     *goal>
     =s =v
     +goal>
     test-num 50
     =s =v
     value t)
  
  (p b50
     =goal>
     test-num 50
     =s =y
     value t
     =imaginal>
     slot2 =s
     ==>
     =imaginal>
     !output! =y
     *goal>
     =s free
     value busy)
 

    #|

(p a50+b50
   =goal>
   test-num 50
   value nil
    =s    nil
   
   =imaginal>
   slot1  =s
   slot2 =s
   
   !safe-bind! =v 1
   ==>
   =imaginal>
   
   !output! =v
   +goal>
   value busy
   =s   free
   *goal>
   =s  =v
   )
|# 


 
  ;; p1 LHS static
  
  (p a51
     =goal>
     test-num 51
     value nil
     color nil
     =imaginal>
     slot1 =s
     !safe-bind! =v 1
     ==>
     =imaginal>
     *goal>
     =s =v
     +goal>
     test-num 51
     =s =v
     value t)
  
  (p b51
     =goal>
     test-num 51
     =s =y
     value t
     =imaginal>
     slot2 =s
     ==>
     =imaginal>
     !output! =y
     *goal>
     =s free
     value busy)
  
    #|

(p a51+b51
   =goal>
   test-num 51
   value nil
    color    nil
   
   =imaginal>
   slot1  =s
   slot2 =s
   
   !safe-bind! =v 1
   ==>
   =imaginal>
   
   !output! =v
   +goal>
   value busy
    =s  free
   *goal>
   =s  =v
   )
|# 


  ;; p1 RHS * static
  
    (p a52
     =goal>
     test-num 52
     value nil
     =s nil
     =imaginal>
     slot1 =s
     !safe-bind! =v 1
     ==>
     =imaginal>
     *goal>
     color =v
     +goal>
     test-num 52
     =s =v
     value t)
  
  (p b52
     =goal>
     test-num 52
     =s =y
     value t
     =imaginal>
     slot2 =s
     ==>
     =imaginal>
     !output! =y
     *goal>
     =s free
     value busy)


    #|

(p a52+b52
   =goal>
   test-num 52
   value nil
    =s    nil
   
   =imaginal>
   slot1  =s
   slot2 =s 
   
   !safe-bind! =v 1
   ==>
   =imaginal>
   
   !output! =v
   +goal>
   value busy
    =s  free
   *goal>
    color =v
   )
|#   
    ;; p1 RHS + static
  
    (p a53
     =goal>
     test-num 53
     value nil
     =s nil
     =imaginal>
     slot1 =s
     !safe-bind! =v 1
     ==>
     =imaginal>
     *goal>
     =s =v
     +goal>
     test-num 53
     color =v
     value t)
  
  (p b53
     =goal>
     test-num 53
     =s =y
     value t
     =imaginal>
     slot2 =s
     ==>
     =imaginal>
     !output! =y
     *goal>
     =s free
     value busy)
 
    #|

(p a53+b53
   =goal>
   test-num 53
   value nil
    =s    nil
   
   =imaginal>
   slot1  =s
   slot2 color
   
   !safe-bind! =v 1
   ==>
   =imaginal>
   
   !output! =v
   +goal>
   value busy
    color  free
   *goal>
   =s  =v
   )
|#  
  ;; p2 LHS static
  
    (p a54
     =goal>
     test-num 54
     value nil
     =s nil
     =imaginal>
     slot1 =s
     !safe-bind! =v 1
     ==>
     =imaginal>
     *goal>
     =s =v
     +goal>
     test-num 54
     =s =v
     value t)
  
  (p b54
     =goal>
     test-num 54
     color =y
     value t
     =imaginal>
     slot2 =s
     ==>
     =imaginal>
     !output! =y
     *goal>
     =s free
     value busy)  
  
    #|

(p a54+b54
   =goal>
   test-num 54
   value nil
    color    nil
   
   =imaginal>
   slot1  color
   slot2 color
   
   !safe-bind! =v 1
   ==>
   =imaginal>
   
   !output! =v
   +goal>
   value busy
    color  free
   *goal>
   color  =v
   )
|# 
  
  ;; p2 RHS static
  
      (p a55
     =goal>
     test-num 55
     value nil
     =s nil
     =imaginal>
     slot1 =s
     !safe-bind! =v 1
     ==>
     =imaginal>
     *goal>
     =s =v
     +goal>
     test-num 55
     =s =v
     value t)
  
  (p b55
     =goal>
     test-num 55
     =s =y
     value t
     =imaginal>
     slot2 =s
     ==>
     =imaginal>
     !output! =y
     *goal>
     color free
     value busy)  
  
      #|

(p a55+b55
   =goal>
   test-num 55
   value nil
   color     nil
   
   =imaginal>
   slot1  color
   slot2 color
   
   !safe-bind! =v 1
   ==>
   =imaginal>
   
   !output! =v
   +goal>
   value busy
   color   free
   *goal>
   color  =v
   )
|# 
  ;; 2 static
  
  ;; p1 LHS and RHS * static
  
      (p a56
     =goal>
     test-num 56
     value nil
     color nil
     =imaginal>
     slot1 =s
     !safe-bind! =v 1
     ==>
     =imaginal>
     *goal>
     color =v
     +goal>
     test-num 56
     =s =v
     value t)
  
  (p b56
     =goal>
     test-num 56
     =s =y
     value t
     =imaginal>
     slot2 =s
     ==>
     =imaginal>
     !output! =y
     *goal>
     =s free
     value busy)
   
  #|

(p a56+b56
   =goal>
   test-num 56
   value nil
   color     nil
   
   =imaginal>
   slot1  =s
   slot2 =s
   
   !safe-bind! =v 1
   ==>
   =imaginal>
   
   !output! =v
   +goal>
   value busy
    =s  free
   *goal>
    color =v
   )
|#  
  
    ;; p1 LHS and RHS + static
  
      (p a57
     =goal>
     test-num 57
     value nil
     color nil
     =imaginal>
     slot1 =s
     !safe-bind! =v 1
     ==>
     =imaginal>
     *goal>
     =s =v
     +goal>
     test-num 57
     color =v
     value t)
  
  (p b57
     =goal>
     test-num 57
     =s =y
     value t
     =imaginal>
     slot2 =s
     ==>
     =imaginal>
     !output! =y
     *goal>
     =s free
     value busy)
    #|

(p a57+b57
   =goal>
   test-num 57
   value nil
    color    nil
   
   =imaginal>
   slot1  =s
   slot2 color
   
   !safe-bind! =v 1
   ==>
   =imaginal>
   
   !output! =v
   +goal>
   value busy
    color  free
   *goal>
    =s =v
   )
|# 
  ;; p1 lhs and p2 lhs static
  
      (p a58
     =goal>
     test-num 58
     value nil
     color nil
     =imaginal>
     slot1 =s
     !safe-bind! =v 1
     ==>
     =imaginal>
     *goal>
     =s =v
     +goal>
     test-num 58
     =s =v
     value t)
  
  (p b58
     =goal>
     test-num 58
     color =y
     value t
     =imaginal>
     slot2 =s
     ==>
     =imaginal>
     !output! =y
     *goal>
     =s free
     value busy) 
  #|
  (p a58+b58
   =goal>
   test-num 58
   value nil
    color    nil
   
   =imaginal>
   slot1  color
   slot2 color
   
   !safe-bind! =v 1
   ==>
   =imaginal>
   
   !output! =v
   +goal>
   value busy
   color  free
   *goal>
   color     =v
   )
|#
  ;; p1 lhs and p2 rhs static
  
       (p a59
     =goal>
     test-num 59
     value nil
     color nil
     =imaginal>
     slot1 =s
     !safe-bind! =v 1
     ==>
     =imaginal>
     *goal>
     =s =v
     +goal>
     test-num 59
     =s =v
     value t)
  
  (p b59
     =goal>
     test-num 59
     =s =y
     value t
     =imaginal>
     slot2 =s
     ==>
     =imaginal>
     !output! =y
     *goal>
     color free
     value busy)   
  #|
(p a59+b59
   =goal>
   test-num 59
   value nil
  color      nil
   
   =imaginal>
   slot1  color
   slot2 color
   
   !safe-bind! =v 1
   ==>
   =imaginal>
   
   !output! =v
   +goal>
   value busy
    color  free
   *goal>
    color  =v
   )
|#
   ;; P1 rhs *,+ static
  
        (p a60
     =goal>
     test-num 60
     value nil
     =s nil
     =imaginal>
     slot1 =s
     !safe-bind! =v 1
     ==>
     =imaginal>
     *goal>
     color =v
     +goal>
     test-num 60
     color =v
     value t)
  
  (p b60
     =goal>
     test-num 60
     =s =y
     value t
     =imaginal>
     slot2 =s
     ==>
     =imaginal>
     !output! =y
     *goal>
     =s free
     value busy)   
#|  
  (p a60+b60
   =goal>
   test-num 60
   value nil
     =s   nil
   
   =imaginal>
   slot1  =s
   slot2 color
   
   !safe-bind! =v 1
   ==>
   =imaginal>
   
   !output! =v
   +goal>
   value busy
    color  free
   *goal>
    color =v
     )
|#

  ;; P1 rhs * and p2 lhs static
  
          (p a61
     =goal>
     test-num 61
     value nil
     =s nil
     =imaginal>
     slot1 =s
     !safe-bind! =v 1
     ==>
     =imaginal>
     *goal>
     color =v
     +goal>
     test-num 61
     =s =v
     value t)
  
  (p b61
     =goal>
     test-num 61
     color =y
     value t
     =imaginal>
     slot2 =s
     ==>
     =imaginal>
     !output! =y
     *goal>
     =s free
     value busy)   
#|
(p a61+b61
   =goal>
   test-num 61
   value nil
    color    nil
   
   =imaginal>
   slot1 color 
   slot2 color
   
   !safe-bind! =v 1
   ==>
   =imaginal>
   
   !output! =v
   +goal>
   value busy
    color  free
   *goal>
   color =v
   )

|#
  ;; p1 rhs * and p2 rhs static
  
         (p a62
     =goal>
     test-num 62
     value nil
     =s nil
     =imaginal>
     slot1 =s
     !safe-bind! =v 1
     ==>
     =imaginal>
     *goal>
     color =v
     +goal>
     test-num 62
     =s =v
     value t)
  
  (p b62
     =goal>
     test-num 62
     =s =y
     value t
     =imaginal>
     slot2 =s
     ==>
     =imaginal>
     !output! =y
     *goal>
     color free
     value busy)   
  
#|
(p a62+b62
   =goal>
   test-num 62
   value nil
    color    nil
   
   =imaginal>
   slot1  color
   slot2 color
   
   !safe-bind! =v 1
   ==>
   =imaginal>
   
   !output! =v
   +goal>
   value busy
    color  free
   *goal>
    color =v
   )

|#
  


    ;; P1 rhs + and p2 lhs static
  
          (p a63
     =goal>
     test-num 63
     value nil
     =s nil
     =imaginal>
     slot1 =s
     !safe-bind! =v 1
     ==>
     =imaginal>
     *goal>
     =s =v
     +goal>
     test-num 63
     color =v
     value t)
  
  (p b63
     =goal>
     test-num 63
     color =y
     value t
     =imaginal>
     slot2 =s
     ==>
     =imaginal>
     !output! =y
     *goal>
     =s free
     value busy)   
  

#|
(p a63+b63
   =goal>
   test-num 63
   value nil
   =s     nil
   
   =imaginal>
   slot1  =s
   slot2 color
   
   !safe-bind! =v 1
   ==>
   =imaginal>
   
   !output! =v
   +goal>
   value busy
   color   free
   *goal>
   =s  =v
   )

|#

  ;; p1 rhs + and p2 rhs static
  
         (p a64
     =goal>
     test-num 64
     value nil
     =s nil
     =imaginal>
     slot1 =s
     !safe-bind! =v 1
     ==>
     =imaginal>
     *goal>
     =s =v
     +goal>
     test-num 64
     color =v
     value t)
  
  (p b64
     =goal>
     test-num 64
     =s =y
     value t
     =imaginal>
     slot2 =s
     ==>
     =imaginal>
     !output! =y
     *goal>
     color free
     value busy)   
  
 #|
(p a64+b64
   =goal>
   test-num 64
   value nil
      =s  nil
   
   =imaginal>
   slot1 =s  
   slot2 color
   
   !safe-bind! =v 1
   ==>
   =imaginal>
   
   !output! =v
   +goal>
   value busy
    color  free
   *goal>
    =s =v
   )

|#
 
    ;; p2 lhs and rhs static
  
          (p a65
     =goal>
     test-num 65
     value nil
     =s nil
     =imaginal>
     slot1 =s
     !safe-bind! =v 1
     ==>
     =imaginal>
     *goal>
     =s =v
     +goal>
     test-num 65
     =s =v
     value t)
  
  (p b65
     =goal>
     test-num 65
     color =y
     value t
     =imaginal>
     slot2 =s
     ==>
     =imaginal>
     !output! =y
     *goal>
     color free
     value busy)   
    
  #|
(p a65+b65
   =goal>
   test-num 65
   value nil
    color    nil
   
   =imaginal>
   slot1  color
   slot2 =s
   
   !safe-bind! =v 1
   ==>
   =imaginal>
   
   !output! =v
   +goal>
   value busy
    color  free
   *goal>
   color     =v
   )

|#

  
    ;; 3 static
  
  
    ;; p1 LHS and RHS * + static
  
      (p a66
     =goal>
     test-num 66
     value nil
     color nil
     =imaginal>
     slot1 =s
     !safe-bind! =v 1
     ==>
     =imaginal>
     *goal>
     color =v
     +goal>
     test-num 66
     color =v
     value t)
  
  (p b66
     =goal>
     test-num 66
     =s =y
     value t
     =imaginal>
     slot2 =s
     ==>
     =imaginal>
     !output! =y
     *goal>
     =s free
     value busy)
 
#|
(p a66+b66
   =goal>
   test-num 66
   value nil
    color   nil
   
   =imaginal>
   slot1  =s
   slot2 color
   
   !safe-bind! =v 1
   ==>
   =imaginal>
   
   !output! =v
   +goal>
   value busy
    color  free
   *goal>
   color  =v
   )

|#
 
  
    ;; p1 LHS RHS *  p2 lhs static
  
      (p a67
     =goal>
     test-num 67
     value nil
     color nil
     =imaginal>
     slot1 =s
     !safe-bind! =v 1
     ==>
     =imaginal>
     *goal>
     color =v
     +goal>
     test-num 67
     =s =v
     value t)
  
  (p b67
     =goal>
     test-num 67
     color =y
     value t
     =imaginal>
     slot2 =s
     ==>
     =imaginal>
     !output! =y
     *goal>
     =s free
     value busy)
  
#|

could there be some trickery in this one?

(p a67+b67
   =goal>
   test-num 67
   value nil
    color    nil
   
   =imaginal>
   slot1 color  
   slot2 color
   
   !safe-bind! =v 1
   ==>
   =imaginal>
   
   !output! =v
   +goal>
   value busy
   color   free
   *goal>
    color =v
   )

|#

  ;; p1 lhs rhs * and p2 rhs static
  
      (p a68
     =goal>
     test-num 68
     value nil
     color nil
     =imaginal>
     slot1 =s
     !safe-bind! =v 1
     ==>
     =imaginal>
     *goal>
     color =v
     +goal>
     test-num 68
     =s =v
     value t)
  
  (p b68
     =goal>
     test-num 68
     =s =y
     value t
     =imaginal>
     slot2 =s
     ==>
     =imaginal>
     !output! =y
     *goal>
     color free
     value busy) 
  
  #|
(p a68+b68
   =goal>
   test-num 68
   value nil
   color     nil
   
   =imaginal>
   slot1  color
   slot2 color
   
   !safe-bind! =v 1
   ==>
   =imaginal>
   
   !output! =v
   +goal>
   value busy
   color   free
   *goal>
    color =v
   )

|#

  ;; p1 lhs rhs + and p2 lhs static
  
       (p a69
     =goal>
     test-num 69
     value nil
     color nil
     =imaginal>
     slot1 =s
     !safe-bind! =v 1
     ==>
     =imaginal>
     *goal>
     =s =v
     +goal>
     test-num 69
     color =v
     value t)
  
  (p b69
     =goal>
     test-num 69
     color =y
     value t
     =imaginal>
     slot2 =s
     ==>
     =imaginal>
     !output! =y
     *goal>
     =s free
     value busy)   
  
 #|
(p a69+b69
   =goal>
   test-num 69
   value nil
   color     nil
   
   =imaginal>
   slot1  =s
   slot2 color
   
   !safe-bind! =v 1
   ==>
   =imaginal>
   
   !output! =v
   +goal>
   value busy
    color  free
   *goal>
   =s  =v
   )

|#
 
   ;; P1 lhs rhs + and p2 rhs static
  
        (p a70
     =goal>
     test-num 70
     value nil
     color nil
     =imaginal>
     slot1 =s
     !safe-bind! =v 1
     ==>
     =imaginal>
     *goal>
     =s =v
     +goal>
     test-num 70
     color =v
     value t)
  
  (p b70
     =goal>
     test-num 70
     =s =y
     value t
     =imaginal>
     slot2 =s
     ==>
     =imaginal>
     !output! =y
     *goal>
     color free
     value busy)   

  #|
(p a70+b70
   =goal>
   test-num 70
   value nil
    color    nil
   
   =imaginal>
   slot1  =s
   slot2 color
   
   !safe-bind! =v 1
   ==>
   =imaginal>
   
   !output! =v
   +goal>
   value busy
    color  free
   *goal>
    =s =v
   )

|#

  
  ;; P1 lhs and p2 lhs rhs static
  
          (p a71
     =goal>
     test-num 71
     value nil
     color nil
     =imaginal>
     slot1 =s
     !safe-bind! =v 1
     ==>
     =imaginal>
     *goal>
     =s =v
     +goal>
     test-num 71
     =s =v
     value t)
  
  (p b71
     =goal>
     test-num 71
     color =y
     value t
     =imaginal>
     slot2 =s
     ==>
     =imaginal>
     !output! =y
     *goal>
     color free
     value busy)   
  
#|
(p a71+b71
   =goal>
   test-num 71
   value nil
    color    nil
   
   =imaginal>
   slot1  color
   slot2 =s
   
   !safe-bind! =v 1
   ==>
   =imaginal>
   
   !output! =v
   +goal>
   value busy
   color   free
   *goal>
   color  =v
   )

|#

  ;; p1 rhs * + and p2 lhs static
  
         (p a72
     =goal>
     test-num 72
     value nil
     =s nil
     =imaginal>
     slot1 =s
     !safe-bind! =v 1
     ==>
     =imaginal>
     *goal>
     color =v
     +goal>
      test-num 72
    color =v
     value t)
  
  (p b72
     =goal>
     test-num 72
     color =y
     value t
     =imaginal>
     slot2 =s
     ==>
     =imaginal>
     !output! =y
     *goal>
     =s free
     value busy)   
 
#|
(p a72+b72
   =goal>
   test-num 72
   value nil
    =s    nil
   
   =imaginal>
   slot1  =s
   slot2 color
   
   !safe-bind! =v 1
   ==>
   =imaginal>
   
   !output! =v
   +goal>
   value busy
   color   free
   *goal>
   color  =v
   )

|#
 
  
    ;; P1 rhs * + and p2 rhs static
  
          (p a73
     =goal>
     test-num 73
     value nil
     =s nil
     =imaginal>
     slot1 =s
     !safe-bind! =v 1
     ==>
     =imaginal>
     *goal>
     color =v
     +goal>
     test-num 73
     color =v
     value t)
  
  (p b73
     =goal>
     test-num 73
     =s =y
     value t
     =imaginal>
     slot2 =s
     ==>
     =imaginal>
     !output! =y
     *goal>
     color free
     value busy)   
  
#|
(p a73+b73
   =goal>
   test-num 73
   value nil
      =s  nil
   
   =imaginal>
   slot1 =s  
   slot2 color
   
   !safe-bind! =v 1
   ==>
   =imaginal>
   
   !output! =v
   +goal>
   value busy
     color free
   *goal>
    color =v
   )

|#



  ;; p1 rhs * and p2 lhs rhs static
  
         (p a74
     =goal>
     test-num 74
     value nil
     =s nil
     =imaginal>
     slot1 =s
     !safe-bind! =v 1
     ==>
     =imaginal>
     *goal>
     color =v
     +goal>
     test-num 74
     =s =v
     value t)
  
  (p b74
     =goal>
     test-num 74
     color =y
     value t
     =imaginal>
     slot2 =s
     ==>
     =imaginal>
     !output! =y
     *goal>
     color free
     value busy)   
  

#|
(p a74+b74
   =goal>
   test-num 74
   value nil
    color    nil
   
   =imaginal>
   slot1 color 
   slot2 =s
   
   !safe-bind! =v 1
   ==>
   =imaginal>
   
   !output! =v
   +goal>
   value busy
    color  free
   *goal>
   color  =v
   )

|#

    
    ;; p1 rhs + and p2 lhs and rhs static
  
          (p a75
     =goal>
     test-num 75
     value nil
     =s nil
     =imaginal>
     slot1 =s
     !safe-bind! =v 1
     ==>
     =imaginal>
     *goal>
     =s =v
     +goal>
     test-num 75
     color =v
     value t)
  
  (p b75
     =goal>
     test-num 75
     color =y
     value t
     =imaginal>
     slot2 =s
     ==>
     =imaginal>
     !output! =y
     *goal>
     color free
     value busy)   
    
  #|
(p a75+b75
   =goal>
   test-num 75
   value nil
    =s    nil
   
   =imaginal>
   slot1  =s
   slot2 =s2
   
   !safe-bind! =v 1
   ==>
   =imaginal>
   
   !output! =v
   +goal>
   value busy
   color   free
   *goal>
    =s =v
   )

|#

  
  
  ;; 4 static
  
   
  
  ;; only p1 lhs dynamic
  
  (p a76
     =goal>
     test-num 76
     value nil
     =s nil
     =imaginal>
     slot1 =s
     !safe-bind! =v 1
     ==>
     =imaginal>
     *goal>
     color =v
     +goal>
     test-num 76
     color =v
     value t)
  
  (p b76
     =goal>
     test-num 76
     color =y
     value t
     =imaginal>
     slot2 =s
     ==>
     =imaginal>
     !output! =y
     *goal>
     color free
     value busy)   
 
#|
(p a76+b76
   =goal>
   test-num 76
   value nil
    =s    nil
   
   =imaginal>
   slot1  =s
   slot2 =s2
   
   !safe-bind! =v 1
   ==>
   =imaginal>
   
   !output! =v
   +goal>
    test-num 76
    value busy
    color  free
   *goal>
   color  =v
   )

|#

     
  ;; only p1 rhs * dynamic
  
   (p a77
     =goal>
     test-num 77
     value nil
     color nil
     =imaginal>
     slot1 =s
     !safe-bind! =v 1
     ==>
     =imaginal>
     *goal>
     =s =v
     +goal>
     test-num 77
     color =v
     value t)
  
  (p b77
     =goal>
     test-num 77
     color =y
     value t
     =imaginal>
     slot2 =s
     ==>
     =imaginal>
     !output! =y
     *goal>
     color free
     value busy)   
  

#|
(p a77+b77
   =goal>
   test-num 77
   value nil
    color    nil
   
   =imaginal>
   slot1  =s
   slot2 =s2
   
   !safe-bind! =v 1
   ==>
   =imaginal>
   
   !output! =v
   +goal>
    test-num 77
  value busy
     color free
   *goal>
   =s  =v
   )

|#

    ;; only p1 rhs + dynamic
  
   (p a78
     =goal>
     test-num 78
     value nil
     color nil
     =imaginal>
     slot1 =s
     !safe-bind! =v 1
     ==>
     =imaginal>
     *goal>
     color =v
     +goal>
     test-num 78
     =s =v
     value t)
  
  (p b78
     =goal>
     test-num 78
     color =y
     value t
     =imaginal>
     slot2 =s
     ==>
     =imaginal>
     !output! =y
     *goal>
     color free
     value busy)   
  #|
(p a78+b78
   =goal>
   test-num 78
   value nil
    color    nil
   
   =imaginal>
   slot1  color
   slot2 =s
   
   !safe-bind! =v 1
   ==>
   =imaginal>
   
   !output! =v
   +goal>
   test-num 78
   value busy
    color  free
   *goal>
    color =v
   )

|#

  
  ;; only p2 lhs dynamic
  
    (p a79
     =goal>
     test-num 79
     value nil
     color nil
     =imaginal>
     slot1 =s
     !safe-bind! =v 1
     ==>
     =imaginal>
     *goal>
     color =v
     +goal>
     test-num 79
     color =v
     value t)
  
  (p b79
     =goal>
     test-num 79
     =s =y
     value t
     =imaginal>
     slot2 =s
     ==>
     =imaginal>
     !output! =y
     *goal>
     color free
     value busy)   
   

#|
(p a79+b79
   =goal>
   test-num 79
   value nil
   color     nil
   
   =imaginal>
   slot1 =s  
   slot2 color
   
   !safe-bind! =v 1
   ==>
   =imaginal>
   
   !output! =v
   +goal>
   test-num 79
   value busy
    color  free
   *goal>
    color =v
   )

|#

 
    ;; only p2 rhs dynamic
  
    (p a80
     =goal>
     test-num 80
     value nil
     color nil
     =imaginal>
     slot1 =s
     !safe-bind! =v 1
     ==>
     =imaginal>
     *goal>
     color =v
     +goal>
      test-num 80
    color =v
     value t)
  
  (p b80
     =goal>
     test-num 80
     color =y
     value t
     =imaginal>
     slot2 =s
     ==>
     =imaginal>
     !output! =y
     *goal>
     =s free
     value busy)   
  
  #|
(p a80+b80
   =goal>
   test-num 80
   value nil
     color   nil
   
   =imaginal>
   slot1  =s
   slot2 color
   
   !safe-bind! =v 1
   ==>
   =imaginal>
   
   !output! =v
   +goal>
   test-num 80
   value busy
    color  free
   *goal>
    color =v
   )

|#

  
  
  
  ;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;
  ;; Tests with 5 possible dynamic location
  ;; LHS,RHS = & LHS,RHS = +  
  
  ;; all 5 dynamic
  
  (p a81
     =goal>
     test-num 81
     value nil
     =s nil
     =imaginal>
     slot1 =s
     !safe-bind! =v 1
     ==>
     =imaginal>
     =goal>
     =s =v
     value t)
  
  (p b81
     =goal>
     test-num 81
     =s =y
     value t
     =imaginal>
     slot2 =s
       !safe-bind! =v 1
   ==>
     =imaginal>
     !output! =y
     =goal>
     =s free
     +goal>
     =s =v
     value busy)
  

#|
(p a81+b81
   =goal>
   test-num 81
   value nil
    =s    nil
   
   =imaginal>
   slot1  =s
   slot2 =s
   
   !safe-bind! =v 1
   ==>
   =imaginal>
   
   !output! =v
   +goal>
   value busy
    =s  =v
   =goal>
     =s free
    value t
   )

|#



  ;; p1 LHS static
  
    (p a82
     =goal>
     test-num 82
     value nil
     color nil
     =imaginal>
     slot1 =s
     !safe-bind! =v 1
     ==>
     =imaginal>
     =goal>
     =s =v
     value t)
  
  (p b82
     =goal>
     test-num 82
     =s =y
     value t
     =imaginal>
     slot2 =s
        !safe-bind! =v 1
  ==>
     =imaginal>
     !output! =y
     =goal>
     =s free
     +goal>
     =s =v
     value busy)
  
#|
(p a82+b82
   =goal>
   test-num 82
   value nil
   color     nil
   
   =imaginal>
   slot1  color
   slot2 color
   
   !safe-bind! =v 1
   ==>
   =imaginal>
   
   !output! =v
   +goal>
   value busy
     color =v
   =goal>
     color free
    value t
   )

|#


  
  ;; p1 RHS  static
  
   (p a83
     =goal>
     test-num 83
     value nil
     =s nil
     =imaginal>
     slot1 =s
     !safe-bind! =v 1
     ==>
     =imaginal>
     =goal>
     color =v
     value t)
  
  (p b83
     =goal>
     test-num 83
     =s =y
     value t
     =imaginal>
     slot2 =s
       !safe-bind! =v 1
   ==>
     =imaginal>
     !output! =y
     =goal>
     =s free
     +goal>
     =s =v
     value busy)
    

#|
(p a83+b83
   =goal>
   test-num 83
   value nil
    color    nil
   
   =imaginal>
   slot1 color 
   slot2 color
   
   !safe-bind! =v 1
   ==>
   =imaginal>
   
   !output! =v
    +goal>
   value busy
     color =v
   =goal>
     color free
    value t
   )

|#


    ;; p2 lhs static
  
    (p a84
     =goal>
     test-num 84
     value nil
     =s nil
     =imaginal>
     slot1 =s
     !safe-bind! =v 1
     ==>
     =imaginal>
     =goal>
     =s =v
     value t)
  
  (p b84
     =goal>
     test-num 84
     color =y
     value t
     =imaginal>
     slot2 =s
       !safe-bind! =v 1
   ==>
     =imaginal>
     !output! =y
     =goal>
     =s free
     +goal>
     =s =v
     value busy)
    


#|
(p a84+b84
   =goal>
   test-num 84
   value nil
     color   nil
   
   =imaginal>
   slot1  color
   slot2 color
   
   !safe-bind! =v 1
   ==>
   =imaginal>
   
   !output! =v
    +goal>
   value busy
   color   =v
   =goal>
    color  free
    value t
   )

|#
  ;; p2 rhs = static
  
  (p a85
     =goal>
     test-num 85
     value nil
     =s nil
     =imaginal>
     slot1 =s
     !safe-bind! =v 1
     ==>
     =imaginal>
     =goal>
     =s =v
     value t)
  
  (p b85
     =goal>
     test-num 85
     =s =y
     value t
     =imaginal>
     slot2 =s
        !safe-bind! =v 1
  ==>
     =imaginal>
     !output! =y
     =goal>
     color free
     +goal>
     =s =v
     value busy)
    
  
#|
(p a85+b85
   =goal>
   test-num 85
   value nil
    color    nil
   
   =imaginal>
   slot1  color
   slot2 color
   
   !safe-bind! =v 1
   ==>
   =imaginal>
   
   !output! =v
    +goal>
   value busy
    color  =v
   =goal>
     color free
    value t
   )

|#

  ;; p2 RHS + static
  
    (p a86
     =goal>
     test-num 86
     value nil
     =s nil
     =imaginal>
     slot1 =s
     !safe-bind! =v 1
     ==>
     =imaginal>
     =goal>
     =s =v
     value t)
  
  (p b86
     =goal>
     test-num 86
     =s =y
     value t
     =imaginal>
     slot2 =s
      !safe-bind! =v 1
    ==>
     =imaginal>
     !output! =y
     =goal>
     =s free
     +goal>
     color =v
     value busy)
   
#|
(p a86+b86
   =goal>
   test-num 86
   value nil
    =s    nil
   
   =imaginal>
   slot1 =s 
   slot2 =s
   
   !safe-bind! =v 1
   ==>
   =imaginal>
   
   !output! =v
    +goal>
   value busy
     color =v
   =goal>
     =s free
    value t
   )

|#
 
  ;; 2 static
  
  ;; p1 LHS and RHS static
  
    (p a87
     =goal>
     test-num 87
     value nil
     color nil
     =imaginal>
     slot1 =s
     !safe-bind! =v 1
     ==>
     =imaginal>
     =goal>
     color =v
     value t)
  
  (p b87
     =goal>
     test-num 87
     =s =y
     value t
     =imaginal>
     slot2 =s
         !safe-bind! =v 1
 ==>
     =imaginal>
     !output! =y
     =goal>
     =s free
     +goal>
     =s =v
     value busy)
 

#|
(p a87+b87
   =goal>
   test-num 87
   value nil
    color    nil
   
   =imaginal>
   slot1 =s 
   slot2 color
   
   !safe-bind! =v 1
   ==>
   =imaginal>
   
   !output! =v
    +goal>
   value busy
     color =v
   =goal>
     color free
    value t
   )

|#   
  
    ;; p1 LHS and p2 lhs static
  
   (p a88
     =goal>
     test-num 88
     value nil
     color nil
     =imaginal>
     slot1 =s
     !safe-bind! =v 1
     ==>
     =imaginal>
     =goal>
     =s =v
     value t)
  
  (p b88
     =goal>
     test-num 88
     color =y
     value t
     =imaginal>
     slot2 =s
        !safe-bind! =v 1
  ==>
     =imaginal>
     !output! =y
     =goal>
     =s free
     +goal>
     =s =v
     value busy)
  
#|
(p a88+b88
   =goal>
   test-num 88
   value nil
    color    nil
   
   =imaginal>
   slot1 color 
   slot2 color
   
   !safe-bind! =v 1
   ==>
   =imaginal>
   
   !output! =v
    +goal>
   value busy
    color  =v
   =goal>
     color free
    value t
   )

|#
  
  ;; p1 lhs and p2 rhs = static
  
  (p a89
     =goal>
     test-num 89
     value nil
     color nil
     =imaginal>
     slot1 =s
     !safe-bind! =v 1
     ==>
     =imaginal>
     =goal>
     =s =v
     value t)
  
  (p b89
     =goal>
     test-num 89
     =s =y
     value t
     =imaginal>
     slot2 =s
       !safe-bind! =v 1
   ==>
     =imaginal>
     !output! =y
     =goal>
     color free
     +goal>
     =s =v
     value busy)
    
  #|
(p a89+b89
   =goal>
   test-num 89
   value nil
    color    nil
   
   =imaginal>
   slot1  color
   slot2 color
   
   !safe-bind! =v 1
   ==>
   =imaginal>
   
   !output! =v
    +goal>
   value busy
     color =v
   =goal>
     color free
    value t
   )

|#
  ;; p1 lhs and p2 rhs + static
  
  (p a90
     =goal>
     test-num 90
     value nil
     color nil
     =imaginal>
     slot1 =s
     !safe-bind! =v 1
     ==>
     =imaginal>
     =goal>
     =s =v
     value t)
  
  (p b90
     =goal>
     test-num 90
     =s =y
     value t
     =imaginal>
     slot2 =s
       !safe-bind! =v 1
   ==>
     =imaginal>
     !output! =y
     =goal>
     =s free
     +goal>
     color =v
     value busy)
  
  #|
(p a90+b90
   =goal>
   test-num 90
   value nil
    color    nil
   
   =imaginal>
   slot1  color
   slot2 color
   
   !safe-bind! =v 1
   ==>
   =imaginal>
   
   !output! =v
    +goal>
   value busy
    color  =v
   =goal>
    color  free
    value t
   )

|#
   ;; P1 rhs = p2 lhs static
  
   (p a91
     =goal>
     test-num 91
     value nil
     =s nil
     =imaginal>
     slot1 =s
     !safe-bind! =v 1
     ==>
     =imaginal>
     =goal>
     color =v
     value t)
  
  (p b91
     =goal>
     test-num 91
     color =y
     value t
     =imaginal>
     slot2 =s
        !safe-bind! =v 1
  ==>
     =imaginal>
     !output! =y
     =goal>
     =s free
     +goal>
     =s =v
     value busy)
    
  #|
(p a91+b91
   =goal>
   test-num 91
   value nil
     color   nil
   
   =imaginal>
   slot1  color
   slot2 color
   
   !safe-bind! =v 1
   ==>
   =imaginal>
   
   !output! =v
    +goal>
   value busy
    color  =v
   =goal>
     color free
    value t
   )

|#
  ;; P1 rhs and p2 rhs = static
  
   (p a92
     =goal>
     test-num 92
     value nil
     =s nil
     =imaginal>
     slot1 =s
     !safe-bind! =v 1
     ==>
     =imaginal>
     =goal>
     color =v
     value t)
  
  (p b92
     =goal>
     test-num 92
     =s =y
     value t
     =imaginal>
     slot2 =s
       !safe-bind! =v 1
   ==>
     =imaginal>
     !output! =y
     =goal>
     color free
     +goal>
     =s =v
     value busy)
   #|
(p a92+b92
   =goal>
   test-num 92
   value nil
      color  nil
   
   =imaginal>
   slot1 color  
   slot2 color
   
   !safe-bind! =v 1
   ==>
   =imaginal>
   
   !output! =v
    +goal>
   value busy
    color  =v
   =goal>
    color  free
    value t
   )

|# 
  ;; p1 rhs and p2 rhs + static
  
   (p a93
     =goal>
     test-num 93
     value nil
     =s nil
     =imaginal>
     slot1 =s
     !safe-bind! =v 1
     ==>
     =imaginal>
     =goal>
     color =v
     value t)
  
  (p b93
     =goal>
     test-num 93
     =s =y
     value t
     =imaginal>
     slot2 =s
        !safe-bind! =v 1
  ==>
     =imaginal>
     !output! =y
     =goal>
     =s free
     +goal>
     color =v
     value busy)
    
   #|
(p a93+b93
   =goal>
   test-num 93
   value nil
    color    nil
   
   =imaginal>
   slot1  color
   slot2 color
   
   !safe-bind! =v 1
   ==>
   =imaginal>
   
   !output! =v
    +goal>
   value busy
    color  =v
   =goal>
     color free
    value t
   )

|#
 ;; p2 lhs and rhs = static
  
   (p a94
     =goal>
     test-num 94
     value nil
     =s nil
     =imaginal>
     slot1 =s
     !safe-bind! =v 1
     ==>
     =imaginal>
     =goal>
     =s =v
     value t)
  
  (p b94
     =goal>
     test-num 94
     color =y
     value t
     =imaginal>
     slot2 =s
        !safe-bind! =v 1
  ==>
     =imaginal>
     !output! =y
     =goal>
     color free
     +goal>
     =s =v
     value busy)
   
  #|
(p a94+b94
   =goal>
   test-num 94
   value nil
    color    nil
   
   =imaginal>
   slot1  color
   slot2 =s
   
   !safe-bind! =v 1
   ==>
   =imaginal>
   
   !output! =v
    +goal>
   value busy
    =s  =v
   =goal>
     color free
    value t
   )

|#
  ;; p2 lhs rhs + static
  
  (p a95
     =goal>
     test-num 95
     value nil
     =s nil
     =imaginal>
     slot1 =s
     !safe-bind! =v 1
     ==>
     =imaginal>
     =goal>
     =s =v
     value t)
  
  (p b95
     =goal>
     test-num 95
     color =y
     value t
     =imaginal>
     slot2 =s
       !safe-bind! =v 1
   ==>
     =imaginal>
     !output! =y
     =goal>
     =s free
     +goal>
     color =v
     value busy)
    
  #|
(p a95+b95
   =goal>
   test-num 95
   value nil
   color     nil
   
   =imaginal>
   slot1 color  
   slot2 color
   
   !safe-bind! =v 1
   ==>
   =imaginal>
   
   !output! =v
    +goal>
   value busy
    color  =v
   =goal>
     color free
    value t
   )

|#
    ;; p2  rhs = + static
  
   (p a96
     =goal>
     test-num 96
     value nil
     =s nil
     =imaginal>
     slot1 =s
     !safe-bind! =v 1
     ==>
     =imaginal>
     =goal>
     =s =v
     value t)
  
  (p b96
     =goal>
     test-num 96
     =s =y
     value t
     =imaginal>
     slot2 =s
        !safe-bind! =v 1
  ==>
     =imaginal>
     !output! =y
     =goal>
     color free
     +goal>
     color =v
     value busy)
    
  #|
(p a96+b96
   =goal>
   test-num 96
   value nil
   color     nil
   
   =imaginal>
   slot1  color
   slot2 color
   
   !safe-bind! =v 1
   ==>
   =imaginal>
   
   !output! =v
    +goal>
   value busy
    color  =v
   =goal>
     color free
    value t
   )

|#
    ;; 3 static
  
  
    ;; p1 LHS and RHS p2 lhs static
  
    (p a97
     =goal>
     test-num 97
     value nil
     color nil
     =imaginal>
     slot1 =s
     !safe-bind! =v 1
     ==>
     =imaginal>
     =goal>
     color =v
     value t)
  
  (p b97
     =goal>
     test-num 97
     color =y
     value t
     =imaginal>
     slot2 =s
       !safe-bind! =v 1
   ==>
     =imaginal>
     !output! =y
     =goal>
     =s free
     +goal>
     =s =v
     value busy)
    
  #|
(p a97+b97
   =goal>
   test-num 83
   value nil
     color   nil
   
   =imaginal>
   slot1  =s
   slot2 color
   
   !safe-bind! =v 1
   ==>
   =imaginal>
   
   !output! =v
    +goal>
   value busy
    color  =v
   =goal>
     color free
    value t
   )

|#
    ;; p1 LHS RHS p2 rhs = static
  
   (p a98
     =goal>
     test-num 98
     value nil
     color nil
     =imaginal>
     slot1 =s
     !safe-bind! =v 1
     ==>
     =imaginal>
     =goal>
     color =v
     value t)
  
  (p b98
     =goal>
     test-num 98
     =s =y
     value t
     =imaginal>
     slot2 =s
      !safe-bind! =v 1
    ==>
     =imaginal>
     !output! =y
     =goal>
     color free
     +goal>
     =s =v
     value busy)
 #|
(p a98+b98
   =goal>
   test-num 98
   value nil
   color     nil
   
   =imaginal>
   slot1 =s 
   slot2 color
   
   !safe-bind! =v 1
   ==>
   =imaginal>
   
   !output! =v
    +goal>
   value busy
    color  =v
   =goal>
     color free
    value t
   )

|#   
  ;; p1 lhs rhs and p2 rhs + static
  
   (p a99
     =goal>
     test-num 99
     value nil
     color nil
     =imaginal>
     slot1 =s
     !safe-bind! =v 1
     ==>
     =imaginal>
     =goal>
     color =v
     value t)
  
  (p b99
     =goal>
     test-num 99
     =s =y
     value t
     =imaginal>
     slot2 =s
       !safe-bind! =v 1
   ==>
     =imaginal>
     !output! =y
     =goal>
     =s free
     +goal>
     color =v
     value busy)
 
#|
(p a99+b99
   =goal>
   test-num 99
   value nil
   color     nil
   
   =imaginal>
   slot1  =s
   slot2 color
   
   !safe-bind! =v 1
   ==>
   =imaginal>
   
   !output! =v
    +goal>
   value busy
     color =v
   =goal>
    color  free
    value t
   )

|#   
  ;; p1 lhs and p2 lhs rhs = static
  
   (p a100
     =goal>
     test-num 100
     value nil
     color nil
     =imaginal>
     slot1 =s
     !safe-bind! =v 1
     ==>
     =imaginal>
     =goal>
     =s =v
     value t)
  
  (p b100
     =goal>
     test-num 100
     color =y
     value t
     =imaginal>
     slot2 =s
      !safe-bind! =v 1
    ==>
     =imaginal>
     !output! =y
     =goal>
     color free
     +goal>
     =s =v
     value busy)
  
  #|
(p a100+b100
   =goal>
   test-num 100
   value nil
   color     nil
   
   =imaginal>
   slot1  color
   slot2 =s
   
   !safe-bind! =v 1
   ==>
   =imaginal>
   
   !output! =v
    +goal>
   value busy
    =s  =v
   =goal>
    color  free
    value t
   )

|#
  
   ;; P1 lhs p2 lhs rhs + static
  
   (p a101
     =goal>
     test-num 101
     value nil
     color nil
     =imaginal>
     slot1 =s
     !safe-bind! =v 1
     ==>
     =imaginal>
     =goal>
     =s =v
     value t)
  
  (p b101
     =goal>
     test-num 101
     color =y
     value t
     =imaginal>
     slot2 =s
      !safe-bind! =v 1
    ==>
     =imaginal>
     !output! =y
     =goal>
     =s free
     +goal>
     color =v
     value busy)
  

  #|
(p a101+b101
   =goal>
   test-num 101
   value nil
    color    nil
   
   =imaginal>
   slot1  color
   slot2 color
   
   !safe-bind! =v 1
   ==>
   =imaginal>
   
   !output! =v
    +goal>
   value busy
    color  =v
   =goal>
     color free
    value t
   )

|#
  
  ;; P1 lhs and p2 rhs = + static
  
   (p a102
     =goal>
     test-num 102
     value nil
     color nil
     =imaginal>
     slot1 =s
     !safe-bind! =v 1
     ==>
     =imaginal>
     =goal>
     =s =v
     value t)
  
  (p b102
     =goal>
     test-num 102
     =s =y
     value t
     =imaginal>
     slot2 =s
     !safe-bind! =v 1
     ==>
     =imaginal>
     !output! =y
     =goal>
     color free
     +goal>
     color =v
     value busy)

    #|
(p a102+b102
   =goal>
   test-num 102
   value nil
    color    nil
   
   =imaginal>
   slot1  color
   slot2 color
   
   !safe-bind! =v 1
   ==>
   =imaginal>
   
   !output! =v
    +goal>
   value busy
    color  =v
   =goal>
    color  free
    value t
   )

|#
  
  ;; p1 rhs and p2 lhs rhs = static
  
   (p a103
     =goal>
     test-num 103
     value nil
     =s nil
     =imaginal>
     slot1 =s
     !safe-bind! =v 1
     ==>
     =imaginal>
     =goal>
     color =v
     value t)
  
  (p b103
     =goal>
     test-num 103
     color =y
     value t
     =imaginal>
     slot2 =s
       !safe-bind! =v 1
   ==>
     =imaginal>
     !output! =y
     =goal>
     color free
     +goal>
     =s =v
     value busy)
 
  #|
(p a103+b103
   =goal>
   test-num 103
   value nil
    color    nil
   
   =imaginal>
   slot1  color
   slot2 =s2
   
   !safe-bind! =v 1
   ==>
   =imaginal>
   
   !output! =v
    +goal>
   value busy
    =s2  =v
   =goal>
    color  free
    value t
   )

|#
   
    ;; P1 rhs p2 lhs rhs + static
  
   (p a104
     =goal>
     test-num 104
     value nil
     =s nil
     =imaginal>
     slot1 =s
     !safe-bind! =v 1
     ==>
     =imaginal>
     =goal>
     color =v
     value t)
  
  (p b104
     =goal>
     test-num 104
     color =y
     value t
     =imaginal>
     slot2 =s
      !safe-bind! =v 1
    ==>
     =imaginal>
     !output! =y
     =goal>
     =s free
     +goal>
     color =v
     value busy)
   
   #|
(p a104+b104
   =goal>
   test-num 104
   value nil
     color   nil
   
   =imaginal>
   slot1  color
   slot2 color
   
   !safe-bind! =v 1
   ==>
   =imaginal>
   
   !output! =v
    +goal>
   value busy
    color  =v
   =goal>
     color free
    value t
   )

|#
 
  ;; p1 rhs and p2 rhs = + static
  
   (p a105
     =goal>
     test-num 105
     value nil
     =s nil
     =imaginal>
     slot1 =s
     !safe-bind! =v 1
     ==>
     =imaginal>
     =goal>
     color =v
     value t)
  
  (p b105
     =goal>
     test-num 105
     =s =y
     value t
     =imaginal>
     slot2 =s
     !safe-bind! =v 1
     ==>
     =imaginal>
     !output! =y
     =goal>
     color free
     +goal>
     color =v
     value busy)
 
  #|
(p a105+b105
   =goal>
   test-num 105
   value nil
    color    nil
   
   =imaginal>
   slot1  color
   slot2 color
   
   !safe-bind! =v 1
   ==>
   =imaginal>
   
   !output! =v
    +goal>
   value busy
    color  =v
   =goal>
    color  free
    value t
   )

|#
   
    
    ;; p2 lhs and rhs = + static
  
   (p a106
     =goal>
     test-num 106
     value nil
     =s nil
     =imaginal>
     slot1 =s
     !safe-bind! =v 1
     ==>
     =imaginal>
     =goal>
     =s =v
     value t)
  
  (p b106
     =goal>
     test-num 106
     color =y
     value t
     =imaginal>
     slot2 =s
      !safe-bind! =v 1
    ==>
     =imaginal>
     !output! =y
     =goal>
     color free
     +goal>
     color =v
     value busy)
    
     #|
(p a106+b106
   =goal>
   test-num 106
   value nil
    color    nil
   
   =imaginal>
   slot1  color
   slot2 =s
   
   !safe-bind! =v 1
   ==>
   =imaginal>
   
   !output! =v
    +goal>
   value busy
    color  =v
   =goal>
    color  free
    value t
   )

|#
 
  
  
  
  ;; 4 static
  
   
  
  ;; only p1 lhs dynamic
  
   (p a107
     =goal>
     test-num 107
     value nil
     =s nil
     =imaginal>
     slot1 =s
     !safe-bind! =v 1
     ==>
     =imaginal>
     =goal>
     color =v
     value t)
  
  (p b107
     =goal>
     test-num 107
     color =y
     value t
     =imaginal>
     slot2 =s
      !safe-bind! =v 1
    ==>
     =imaginal>
     !output! =y
     =goal>
     color free
     +goal>
     color =v
     value busy)
    
    #|
(p a107+b107
   =goal>
   test-num 107
   value nil
     color   nil
   
   =imaginal>
   slot1  color
   slot2 =s2
   
   !safe-bind! =v 1
   ==>
   =imaginal>
   
   !output! =v
    +goal>
   value busy
     color =v
   =goal>
    color  free
    value t
   )

|#
    
  ;; only p1 rhs  dynamic
  
   (p a108
     =goal>
     test-num 108
     value nil
     color nil
     =imaginal>
     slot1 =s
     !safe-bind! =v 1
     ==>
     =imaginal>
     =goal>
     =s =v
     value t)
  
  (p b108
     =goal>
     test-num 108
     color =y
     value t
     =imaginal>
     slot2 =s
      !safe-bind! =v 1
    ==>
     =imaginal>
     !output! =y
     =goal>
     color free
     +goal>
     color =v
     value busy)

    #|
(p a108+b108
   =goal>
   test-num 108
   value nil
   color     nil
   
   =imaginal>
   slot1  color
   slot2 =s
   
   !safe-bind! =v 1
   ==>
   =imaginal>
   
   !output! =v
    +goal>
   value busy
     color =v
   =goal>
     color free
    value t
   )

|#
  
    ;; only p2 lhs dynamic
  
   (p a109
     =goal>
     test-num 109
     value nil
     color nil
     =imaginal>
     slot1 =s
     !safe-bind! =v 1
     ==>
     =imaginal>
     =goal>
     color =v
     value t)
  
  (p b109
     =goal>
     test-num 109
     =s =y
     value t
     =imaginal>
     slot2 =s
     !safe-bind! =v 1
     ==>
     =imaginal>
     !output! =y
     =goal>
     color free
     +goal>
     color =v
     value busy)
    
    #|
(p a109+b109
   =goal>
   test-num 109
   value nil
    color    nil
   
   =imaginal>
   slot1  =s
   slot2 color
   
   !safe-bind! =v 1
   ==>
   =imaginal>
   
   !output! =v
    +goal>
   value busy
    color  =v
   =goal>
    color  free
    value t
   )

|#

  ;; only p2 rhs = dynamic
  
   (p a110
     =goal>
     test-num 110
     value nil
     color nil
     =imaginal>
     slot1 =s
     !safe-bind! =v 1
     ==>
     =imaginal>
     =goal>
     color =v
     value t)
  
  (p b110
     =goal>
     test-num 110
     color =y
     value t
     =imaginal>
     slot2 =s
     !safe-bind! =v 1
     ==>
     =imaginal>
     !output! =y
     =goal>
     =s free
     +goal>
     color =v
     value busy)
   

  #|
(p a110+b110
   =goal>
   test-num 110
   value nil
    color    nil
   
   =imaginal>
   slot1 =s 
   slot2 color
   
   !safe-bind! =v 1
   ==>
   =imaginal>
   
   !output! =v
    +goal>
   value busy
    color  =v
   =goal>
     color free
    value t
   )

|#

   
    ;; only p2 rhs + dynamic
  
    (p a111
     =goal>
     test-num 111
     value nil
     color nil
     =imaginal>
     slot1 =s
     !safe-bind! =v 1
     ==>
     =imaginal>
     =goal>
     color =v
     value t)
  
  (p b111
     =goal>
     test-num 111
     color =y
     value t
     =imaginal>
     slot2 =s
     !safe-bind! =v 1
     ==>
     =imaginal>
     !output! =y
     =goal>
     color free
     +goal>
     =s =v
     value busy)
    
  
    #|
(p a111+b111
   =goal>
   test-num 111
   value nil
   color     nil
   
   =imaginal>
   slot1 =s  
   slot2 =s2
   
   !safe-bind! =v 1
   ==>
   =imaginal>
   
   !output! =v
    +goal>
   value busy
    =s2  =v
   =goal>
    color  free
    value t
   )

|#

  ;;; p1 lhs = with p2 = , =
  
  ;; all dynamic
  
  (p a112
     =goal>
     test-num 112
     value nil
     =s nil
     =imaginal>
     slot1 =s
     test-num 112
     ==>
     =imaginal>
     test-num t
     )
  
  (p b112
     =goal>
     test-num 112
     =s nil
     =imaginal>
     test-num t
     slot2 =s
     !safe-bind! =v 1
     ==>
     =imaginal>
     =goal>
     =s =v)
  
    #|


(p a112+b112
   =goal>
   test-num 112
   value nil
    =s    nil
    =s1   nil
   =imaginal>
   slot1  =s
   slot2 =s1
   test-num 112
   
   !safe-bind! =v 1
   ==>
   =imaginal>
   test-num t
   =goal>
    =s1  =v
   )

|#

  ;; one static
  
  ;; p1 lhs static
  
  (p a113
     =goal>
     test-num 113
     value nil
     color nil
     =imaginal>
     slot1 =s
     test-num 113
     ==>
     =imaginal>
     test-num t
     )
  
  (p b113
     =goal>
     test-num 113
     =s nil
     =imaginal>
     test-num t
     slot2 =s
     !safe-bind! =v 1
     ==>
     =imaginal>
     =goal>
     =s =v)

    #|

   This is what's produced, but should it be able to 
   note that =s2 is color and remove the redundant test
   forcing an instantiation of =s2 ?


(p a113+b113
   =goal>
   test-num 113
   value nil
    =s2    nil
    color nil

   =imaginal>
   slot1  =s
   slot2 =s2
   test-num 113
   
   !safe-bind! =v 1
   ==>
   =imaginal>
   test-num t
    =goal>
    =s2  =v
   )

|#



  ;; p2 lhs static
  
    (p a114
     =goal>
     test-num 114
     value nil
     =s nil
     =imaginal>
     slot1 =s
     test-num 114
     ==>
     =imaginal>
     test-num t
     )
  
  (p b114
     =goal>
     test-num 114
     color nil
     =imaginal>
     test-num t
     slot2 =s
     !safe-bind! =v 1
     ==>
     =imaginal>
     =goal>
     =s =v)
  
      #|

   This is what's produced, but should it be able to 
   note that =s is color and remove the redundant test
   forcing an instantiation of =s ?

(p a114+b114
   =goal>
   test-num 114
   value nil
     color   nil
   =s nil

   =imaginal>
   slot1  =s
   slot2 =s1
   test-num 114
   
   !safe-bind! =v 1
   ==>
   =imaginal>
   test-num t
   =goal>
   =s1   =v
   )

|#

;; p2 rhs static
  
    (p a115
     =goal>
     test-num 115
     value nil
     =s nil
     =imaginal>
     slot1 =s
     test-num 115
     ==>
     =imaginal>
     test-num t
     )
  
  (p b115
     =goal>
     test-num 115
     =s nil
     =imaginal>
     test-num t
     slot2 =s
     !safe-bind! =v 1
     ==>
     =imaginal>
     =goal>
     color =v)
  

    #|
(p a115+b115
   =goal>
   test-num 115
   value nil
   =s    nil
   =s2 nil
   =imaginal>
   slot1  =s
   slot2 =s2
   test-num 115
   
   !safe-bind! =v 1
   ==>
   =imaginal>
   test-num t
   =goal>
    color  =v
   )

|#
  ;; two static
  
  ;; p1 lhs p2 lhs
  
    (p a116
     =goal>
     test-num 116
     value nil
     color nil
     =imaginal>
     slot1 =s
     test-num 116
     ==>
     =imaginal>
     test-num t
     )
  
  (p b116
     =goal>
     test-num 116
     color nil
     =imaginal>
     test-num t
     slot2 =s
     !safe-bind! =v 1
     ==>
     =imaginal>
     =goal>
     =s =v)
  
    #|
(p a116+b116
   =goal>
   test-num 116
   value nil
    color    nil
   
   =imaginal>
   slot1  =s
   slot2 =s1
   test-num 116
   
   !safe-bind! =v 1
   ==>
   =imaginal>
   test-num t
   =goal>
    =s1  =v
   )

|#


  ;; p1 lhs p2 rhs
  
    (p a117
     =goal>
     test-num 117
     value nil
     color nil
     =imaginal>
     slot1 =s
     test-num 117
     ==>
     =imaginal>
     test-num t
     )
  
  (p b117
     =goal>
     test-num 117
     =s nil
     =imaginal>
     test-num t
     slot2 =s
     !safe-bind! =v 1
     ==>
     =imaginal>
     =goal>
     color =v)
  
    #|
(p a117+b117
   =goal>
   test-num 117
   value nil
    color    nil
   =s2  nil
   =imaginal>
   slot1 =s 
   slot2 =s2
   test-num 117
   
   !safe-bind! =v 1
   ==>
   =imaginal>
   test-num t
   =goal>
    color  =v
   )

|#


  ;; p2 lhs rhs
  
    (p a118
     =goal>
     test-num 118
     value nil
     =s nil
     =imaginal>
     slot1 =s
     test-num 118
     ==>
     =imaginal>
     test-num t
     )
  
  (p b118
     =goal>
     test-num 118
     color nil
     =imaginal>
     test-num t
     slot2 =s
     !safe-bind! =v 1
     ==>
     =imaginal>
     =goal>
     color =v)

      #|
(p a118+b118
   =goal>
   test-num 118
   value nil
    color    nil
    =s  nil
   =imaginal>
   slot1  =s
   slot2 =s1
   test-num 118
   
   !safe-bind! =v 1
   ==>
   =imaginal>
   test-num t
   =goal>
    color  =v
   )

|#
  
  
  ;;; p1 rhs +  with p2 =, =
  
  
  (p a119
     =imaginal>
     slot1 =s
     test-num 119
     ==>
     =imaginal>
     test-num t
     +goal>
     =s 119
     )
  
  (p b119
     =goal>
     =s 119
     =imaginal>
     test-num t
     slot2 =s
     ==>
     =imaginal>
     =goal>
     =s free)
  
    #|
(p a119+b119
   
   =imaginal>
   slot1  =s
   slot2 =s
   test-num 119
   
   ==>
   =imaginal>
   test-num t
  
   +goal>
    =s  free
   )

|#

  
  ;; one static
  
  ;; p1 rhs
  
  
    (p a120
     =imaginal>
     slot1 =s
     test-num 120
     ==>
     =imaginal>
     test-num t
     +goal>
     color 120
     )
  
  (p b120
     =goal>
     =s 120
     =imaginal>
     test-num t
     slot2 =s
     ==>
     =imaginal>
     =goal>
     =s free)
  
    #|
(p a120+b120
   
   =imaginal>
   slot1  =s
   slot2 color
   test-num 120
   
   ==>
   =imaginal>
   test-num t
  
   +goal>
    color  free
   )

|#


  ;; p2 lhs
  
    (p a121
     =imaginal>
     slot1 =s
     test-num 121
     ==>
     =imaginal>
     test-num t
     +goal>
     =s 121
     )
  
  (p b121
     =goal>
     color 121
     =imaginal>
     test-num t
     slot2 =s
     ==>
     =imaginal>
     =goal>
     =s free)
  
    #|


(p a121+b121
   
   =imaginal>
   slot1  color
   slot2 color
   test-num 121
   
   ==>
   =imaginal>
   test-num t
  
   +goal>
   color free
   )

|#


  ;; p2 rhs
  
    (p a122
     =imaginal>
     slot1 =s
     test-num 122
     ==>
     =imaginal>
     test-num t
     +goal>
     =s 122
     )
  
  (p b122
     =goal>
     =s 122
     =imaginal>
     test-num t
     slot2 =s
     ==>
     =imaginal>
     =goal>
     color free)
  

    #|
(p a122+b122
   
   =imaginal>
   slot1 color
   slot2 color
   test-num 122
   
   ==>
   =imaginal>
   test-num t
  
   +goal>
   color   free
   )

|#


  ;; two static
  
  ;; p1 rhs p2 lhs
  
    (p a123
     =imaginal>
     slot1 =s
     test-num 123
     ==>
     =imaginal>
     test-num t
     +goal>
     color 123
     )
  
  (p b123
     =goal>
     color 123
     =imaginal>
     test-num t
     slot2 =s
     ==>
     =imaginal>
     =goal>
     =s free)
  
    #|
(p a123+b123
   
   =imaginal>
   slot1 =s 
   slot2 color 
   test-num 123
   
   ==>
   
   =imaginal>
   test-num t
  
   +goal>
     color free
   )

|#



  ;; p1 rhs p2 rhs
  
    (p a124
     =imaginal>
     slot1 =s
     test-num 124
     ==>
     =imaginal>
     test-num t
     +goal>
     color 124
     )
  
  (p b124
     =goal>
     =s 124
     =imaginal>
     test-num t
     slot2 =s
     ==>
     =imaginal>
     =goal>
     color free)
  
    #|
(p a124+b124
   
   =imaginal>
   slot1 =s 
   slot2 color 
   test-num 124
   
   ==>
   
   =imaginal>
   test-num t
  
   +goal>
     color free
   )

|#


  ;; p2 lhs rhs
  
    (p a125
     =imaginal>
     slot1 =s
     test-num 125
     ==>
     =imaginal>
     test-num t
     +goal>
     =s 125
     )
  
  (p b125
     =goal>
     color 125
     =imaginal>
     test-num t
     slot2 =s
     ==>
     =imaginal>
     =goal>
     color free)
  
  
      #|
(p a125+b125
   
   =imaginal>
   slot1 color 
   slot2 =s 
   test-num 125
   
   ==>
   
   =imaginal>
   test-num t
  
   +goal>
     color free
   )

|#



  
  ;;; Here's a test with multiple dynamics
  
  ;(goal-focus-fct (car (define-chunks (value 1))))
  ;(set-buffer-chunk 'imaginal (car (define-chunks (slot1 color slot2 color slot3 value))))
  
  ;; all dynamic
  
  (p a126
     =goal>
     =s3 1
     =s1 nil
     test-num 126
     =imaginal>
     slot1 =s1
     slot3 =s3
     ==>
     =goal>
     =s1 blue
     =imaginal>)

  (p b126
     =goal>
     test-num 126
     =s2 blue
     =imaginal>
     slot2 =s2
     slot3 =s3
     ==>
     =goal>
     =s2 nil
     =s3 2)



#|

(p a126+b126
   =goal>
   test-num 126
    =s3  1
    =s1  nil
   =imaginal>
   slot1 =s1
   slot2 =s1
   slot3 =s3
   ==>
   =goal>
    =s1  nil
    =s3  2
  )
|#

  ;; make p2 lhs static
  
  (p a127
     =goal>
     test-num 127
     =s3 1
     =s1 nil
     =imaginal>
     slot1 =s1
     slot3 =s3
     ==>
     =goal>
     =s1 blue
     =imaginal>)
  
  (p b127
     =goal>
     test-num 127
     color blue
     =imaginal>
     slot2 =s2
     slot3 =s3
     ==>
     =goal>
     =s2 nil
     =s3 2)

 #|

(p a127+b127
   =goal>
   test-num 127
    =s3  1
   color   nil
   =imaginal>
   slot1 color
   slot2 color
   slot3 =s3
   ==>
   =imaginal>
   =goal>
    color  nil
    =s3  2
  )
|#
 
  
  
  ;;; Test case where dynamics don't match between productions
  
    ;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;
  ;; Tests with 4 possible dynamic location
  ;; LHS,RHS = & LHS,RHS =
  
  ;; slot4 has value slot4
  
  ;; all 4 dynamic
  
  (p a128
     =goal>
     test-num 128
     value nil
     =s1 nil
     
     =imaginal>
     slot1 =s1
     
     !safe-bind! =v 1
     ==>
     =imaginal>
     =goal>
     =s1 =v
     value t)
  
  (p b128
     =goal>
     test-num 128
     =s4 nil
     value t
     =imaginal>
     slot4 =s4
     ==>
     =imaginal>
     =goal>
     =s4 free
     value busy)
  

#|
(p a128+b128
   =goal>
   test-num 128
   value nil
   =s1     nil
   =s4     nil
   =imaginal>
    slot1 =s1
   slot4 =s4
   !safe-bind! =v 1
   ==>
   =imaginal>
   =goal>
   value busy
   =s1 =v
   =s4 free)
|#

  ;; p1 LHS static
  
    (p a129
     =goal>
     test-num 129
     value nil
     color nil
     
     =imaginal>
     slot1 =s1
     
     !safe-bind! =v 1
     ==>
     =imaginal>
     =goal>
     =s1 =v
     value t)
  
  (p b129
     =goal>
     test-num 129
     =s4 nil
     value t
     =imaginal>
     slot4 =s4
     ==>
     =imaginal>
     =goal>
     =s4 free
     value busy)
  
#|
(p a129+b129
   =goal>
   test-num 129
   value nil
   color     nil
   =s4     nil
   =imaginal>
    slot1 =s1
   slot4 =s4
   !safe-bind! =v 1
   ==>
   =imaginal>
   =goal>
   value busy
   =s1 =v
   =s4 free)
|#

  
  ;; p1 RHS static
  
    (p a130
     =goal>
     test-num 130
     value nil
     =s1 nil
     
     =imaginal>
     slot1 =s1
     
     !safe-bind! =v 1
     ==>
     =imaginal>
     =goal>
     color =v
     value t)
  
  (p b130
     =goal>
     test-num 130
     =s4 nil
     value t
     =imaginal>
     slot4 =s4
     ==>
     =imaginal>
     =goal>
     =s4 free
     value busy)
  

#|
(p a130+b130
   =goal>
   test-num 130
   value nil
   =s1     nil
   =s4     nil
   =imaginal>
    slot1 =s1
   slot4 =s4
   !safe-bind! =v 1
   ==>
   =imaginal>
   =goal>
   value busy
   color =v
   =s4 free)
|#

  
  ;; p2 LHS static
  
    (p a131
     =goal>
     test-num 131
     value nil
     =s1 nil
     
     =imaginal>
     slot1 =s1
     
     !safe-bind! =v 1
     ==>
     =imaginal>
     =goal>
     =s1 =v
     value t)
  
  (p b131
     =goal>
     test-num 131
     slot4 nil
     value t
     =imaginal>
     slot4 =s4
     ==>
     =imaginal>
     =goal>
     =s4 free
     value busy)
    
#|
(p a131+b131
   =goal>
   test-num 131
   value nil
   =s1     nil
   slot4     nil
   =imaginal>
    slot1 =s1
   slot4 =s4
   !safe-bind! =v 1
   ==>
   =imaginal>
   =goal>
   value busy
   =s1 =v
   =s4 free)
|#



  ;; p2 RHS static
  
   (p a132
     =goal>
     test-num 132
     value nil
     =s1 nil
     
     =imaginal>
     slot1 =s1
     
     !safe-bind! =v 1
     ==>
     =imaginal>
     =goal>
     =s1 =v
     value t)
  
  (p b132
     =goal>
     test-num 132
     =s4 nil
     value t
     =imaginal>
     slot4 =s4
     ==>
     =imaginal>
     =goal>
     slot4 free
     value busy)
    
#|
(p a132+b132
   =goal>
   test-num 132
   value nil
   =s1     nil
   =s4     nil
   =imaginal>
    slot1 =s1
   slot4 =s4
   !safe-bind! =v 1
   ==>
   =imaginal>
   =goal>
   value busy
   =s1 =v
   slot4 free)
|#
  


  ;; p1 LHS and RHS static
  
   (p a133
     =goal>
     test-num 133
     value nil
     color nil
     
     =imaginal>
     slot1 =s1
     !safe-bind! =v 1
     ==>
     =imaginal>
     =goal>
     color =v
     value t)
  
  (p b133
     =goal>
     test-num 133
     =s4 nil
     value t
     =imaginal>
     slot4 =s4
     ==>
     =imaginal>
     =goal>
     =s4 free
     value busy)
   
#|
(p a133+b133
   =goal>
   test-num 133
   value nil
   color     nil
   =s4     nil
   =imaginal>
    slot1 =s1
   slot4 =s4
   !safe-bind! =v 1
   ==>
   =imaginal>
   =goal>
   value busy
   color =v
   =s4 free)
|#

 
  ;; p1 lhs and p2 lhs static
  
    (p a134
     =goal>
     test-num 134
     value nil
     color nil
     
     =imaginal>
     slot1 =s1
     
     !safe-bind! =v 1
     ==>
     =imaginal>
     =goal>
     =s1 =v
     value t)
  
  (p b134
     =goal>
     test-num 134
     slot4 nil
     value t
     =imaginal>
     slot4 =s4
     ==>
     =imaginal>
     =goal>
     =s4 free
     value busy)
    
  
#|
(p a134+b134
   =goal>
   test-num 134
   value nil
   color     nil
   slot4     nil
   =imaginal>
    slot1 =s1
   slot4 =s4
   !safe-bind! =v 1
   ==>
   =imaginal>
   =goal>
   value busy
   =s1 =v
   =s4 free)
|#

  
  ;; p1 lhs and p2 rhs static
  
     (p a135
     =goal>
     test-num 135
     value nil
     color nil
     
     =imaginal>
     slot1 =s1
     
     !safe-bind! =v 1
     ==>
     =imaginal>
     =goal>
     =s1 =v
     value t)
  
  (p b135
     =goal>
     test-num 135
     =s4 nil
     value t
     =imaginal>
     slot4 =s4
     ==>
     =imaginal>
     =goal>
     slot4 free
     value busy)
   
 #|
(p a135+b135
   =goal>
   test-num 135
   value nil
   color     nil
   =s4     nil
   =imaginal>
    slot1 =s1
   slot4 =s4
   !safe-bind! =v 1
   ==>
   =imaginal>
   =goal>
   value busy
   =s1 =v
   slot4 free)
|#
 
  
  ;; P1 rhs and p2 lhs static
  
  (p a136
     =goal>
     test-num 136
     value nil
     =s1 nil
     
     =imaginal>
     slot1 =s1
     
     !safe-bind! =v 1
     ==>
     =imaginal>
     =goal>
     color =v
     value t)
  
  (p b136
     =goal>
     test-num 136
     slot4 nil
     value t
     =imaginal>
     slot4 =s4
     ==>
     =imaginal>
     =goal>
     =s4 free
     value busy)
#|
(p a136+b136
   =goal>
   test-num 136
   value nil
   =s1     nil
   slot4     nil
   =imaginal>
    slot1 =s1
   slot4 =s4
   !safe-bind! =v 1
   ==>
   =imaginal>
   =goal>
   value busy
   color =v
   =s4 free)
|#
    
  ;; p1 rhs and p2 rhs static
  
   (p a137
     =goal>
     test-num 137
     value nil
     =s1 nil
     
     =imaginal>
     slot1 =s1
     
     !safe-bind! =v 1
     ==>
     =imaginal>
     =goal>
     color =v
     value t)
  
  (p b137
     =goal>
     test-num 137
     =s4 nil
     value t
     =imaginal>
     slot4 =s4
     ==>
     =imaginal>
     =goal>
     slot4 free
     value busy)

#|
(p a137+b137
   =goal>
   test-num 137
   value nil
   =s1     nil
   =s4     nil
   =imaginal>
    slot1 =s1
   slot4 =s4
   !safe-bind! =v 1
   ==>
   =imaginal>
   =goal>
   value busy
   color =v
   slot4 free)
|#
    
    ;; p2 lhs and rhs static
  
   (p a138
     =goal>
     test-num 138
     value nil
     =s1 nil
     
     =imaginal>
     slot1 =s1
     
     !safe-bind! =v 1
     ==>
     =imaginal>
     =goal>
     =s1 =v
     value t)
  
  (p b138
     =goal>
     test-num 138
     slot4 nil
     value t
     =imaginal>
     slot4 =s4
     ==>
     =imaginal>
     =goal>
     slot4 free
     value busy)

   #|
(p a138+b138
   =goal>
   test-num 138
   value nil
   =s1     nil
   slot4     nil
   =imaginal>
    slot1 =s1
   slot4 =s4
   !safe-bind! =v 1
   ==>
   =imaginal>
   =goal>
   value busy
   =s1 =v
   slot4 free)
|#
 
  
  ;; only p1 lhs dynamic
  
   (p a139
     =goal>
     test-num 139
     value nil
     =s1 nil
     
     =imaginal>
     slot1 =s1
     
     !safe-bind! =v 1
     ==>
     =imaginal>
     =goal>
     color =v
     value t)
  
  (p b139
     =goal>
     test-num 139
     slot4 nil
     value t
     =imaginal>
     slot4 =s4
     ==>
     =imaginal>
     =goal>
     slot4 free
     value busy)
    
  #|
(p a139+b139
   =goal>
   test-num 139
   value nil
   =s1     nil
   slot4     nil
   =imaginal>
    slot1 =s1
   slot4 =s4
   !safe-bind! =v 1
   ==>
   =imaginal>
   =goal>
   value busy
   color =v
   slot4 free)
|#

  ;; only p1 rhs dynamic
  
   (p a140
     =goal>
     test-num 140
     value nil
     color nil
     
     =imaginal>
     slot1 =s1
     
     !safe-bind! =v 1
     ==>
     =imaginal>
     =goal>
     =s1 =v
     value t)
  
  (p b140
     =goal>
     test-num 140
     slot4 nil
     value t
     =imaginal>
     slot4 =s4
     ==>
     =imaginal>
     =goal>
     slot4 free
     value busy)

#|
(p a140+b140
   =goal>
   test-num 140
   value nil
   color     nil
   slot4     nil
   =imaginal>
    slot1 =s1
   slot4 =s4
   !safe-bind! =v 1
   ==>
   =imaginal>
   =goal>
   value busy
   =s1 =v
   slot4 free)
|#

    
    ;; only p2 lhs dynamic
  
   (p a141
     =goal>
     test-num 141
     value nil
     color nil
     
     =imaginal>
     slot1 =s1
     
     !safe-bind! =v 1
     ==>
     =imaginal>
     =goal>
     color =v
     value t)
  
  (p b141
     =goal>
     test-num 141
     =s4 nil
     value t
     =imaginal>
     slot4 =s4
     ==>
     =imaginal>
     =goal>
     slot4 free
     value busy)
   
#|
(p a141+b141
   =goal>
   test-num 141
   value nil
   color     nil
   =s4     nil
   =imaginal>
    slot1 =s1
   slot4 =s4
   !safe-bind! =v 1
   ==>
   =imaginal>
   =goal>
   value busy
   color =v
   slot4 free)
|#

 
    ;; only p2 rhs dynamic
  
    (p a142
     =goal>
     test-num 142
     value nil
     color nil
     
     =imaginal>
     slot1 =s1
     
     !safe-bind! =v 1
     ==>
     =imaginal>
     =goal>
     color =v
     value t)
  
  (p b142
     =goal>
     test-num 142
     slot4 nil
     value t
     =imaginal>
     slot4 =s4
     ==>
     =imaginal>
     =goal>
     =s4 free
     value busy)
  
  
#|
(p a142+b142
   =goal>
   test-num 142
   value nil
   color     nil
   slot4     nil
   =imaginal>
    slot1 =s1
   slot4 =s4
   !safe-bind! =v 1
   ==>
   =imaginal>
   =goal>
   value busy
   color =v
   =s4 free)
|#
  
  
  ;;; test dynamic slot from a bind variable
  
  
  
    (p a143
     =goal>
     test-num 143
     value nil
     =s1 nil
     
     =imaginal>
     slot1 =s1
     
       !safe-bind! =v 1
       !safe-bind! =s2 'color
     ==>
     =imaginal>
     =goal>
     =s2 =v
     value t)
  
  (p b143
     =goal>
     test-num 143
     =s1 =x
     value t
     
     =imaginal>
     slot1 =s1

     !safe-bind! =s2 'value
     ==>
     =imaginal>
     =goal>
     =s1 free
     =s2 busy)
  

#|

   Seems like =s2 should be instantiated, but can't
   because of the bind so an eval would need to be added but
   that's not an easy/feasible option...
   As-is it really depends on the determinism of the safe-bind

(p a143+b143
   =goal>
   test-num 143
   value nil
   =s1 nil
   =imaginal>
   slot1 =s1
   !safe-bind! =v 1
   !safe-bind! =s2 'color
   !safe-bind! =s3 'value
   ==>
   =imaginal>
   =goal>
   =s3 busy
   =s1 free)
|#

    
  


;;; This is case 143 where in a =s1 is not =s2 and a different var occurs in b

  
    (p a144
     =goal>
     test-num 144
     value nil
     =s1 nil
     
     =imaginal>
     slot1 =s1
     
       !safe-bind! =v 1
       !safe-bind! =s2 'slot4
     ==>
     =imaginal>
     =goal>
     =s2 =v
     value t)
  
  (p b144
     =goal>
     test-num 144
     =s4 =x
     value t
     
     =imaginal>
     slot4 =s4
     slot1 =s1
     !safe-bind! =s2 'value
     ==>
     =imaginal>
     =goal>
     =s1 free
     =s2 busy)
  

#|
(p a144+b144
   =goal>
   test-num 144
   value nil
   =s1 nil
   
   =imaginal>
   slot1 =s1
   slot4 =s4
   !safe-bind! =v 1
   !safe-bind! =s2 'slot4
   !safe-bind! =s3 'value
   ==>
   =imaginal>
   =goal>
   =s2 =v
   =s3 busy
   =s1 free)
|#


   (p a145
       =goal>
       test-num 145
       value nil
       !safe-bind! =v 'value
       ==>
       =goal>
       value t)

     (p b145
       =goal>
        value t
       test-num 145
       !safe-bind! =v 'value
       ==>
       =goal>
        =v 3)

#|

(p a145+b145
   =goal>
   test-num 145
   value nil
   !safe-bind! =v 'value
   ==>
   =goal>
   =v 3)

|#

   (p a146
     =goal>
     test-num 146
     value nil
     !safe-bind! =v 'value
     ==>
     =goal>
      value t
      color blue)

  (p b146
    =goal>
     test-num 146
     color blue
    !safe-bind! =v 'value
   ==>
    =goal>
     color red
      =v 3)

#|

(p a146+b146
   =goal>
   test-num 146
   value nil
   !safe-bind! =v 'value
   ==>
   =goal>
   color red
   =v 3)

|#




;;; swap goal and imaginal and run them all again

  
  ;;;;;;;;;;;;;;;;;;;;;;;
  ;; simple case of a p1 RHS = & p2 LHS dynamic
  
  ;; both dynamic
  
  (p c1
     =imaginal>
     test-num 1
     value nil
     =goal>
     slot1 =s
     !safe-bind! =v 1
     ==>
     =goal>
     =imaginal>
     =s =v
     value t)
  
  (p d1
     =imaginal>
     test-num 1
     =s =y
     value t
     =goal>
     slot1 =s
     ==>
     =goal>
     !output! =y
     =imaginal>
     value busy)
  
  
  #|
   (p c1+d1
      =imaginal>
      test-num 1
      value nil
      =goal>
      slot1 =s
      !safe-bind! =v 1
      ==>
      =goal>
      !output! =v
      =imaginal>
      value busy
      =s =v)
      
  |#
  
  
  ;; only p1 RHS dynamic
  
    (p c2
     =imaginal>
     test-num 2
     value nil
     =goal>
     slot1 =s
     !safe-bind! =v 1
     ==>
     =goal>
     =imaginal>
     =s =v
     value t)
  
  (p d2
     =imaginal>
     test-num 2
     color =y
     value t
     =goal>
     slot1 =s
     ==>
     =goal>
     !output! =y
     =imaginal>
     value busy)
  
  
  #| (p c2+d2
       =imaginal>
         test-num 2
         value nil
       =goal>
          slot1 color
       !safe-bind! =v 1
      ==>
       =goal>
       !output! =v
       =imaginal>
       color =v
       value busy)
  |#
  
  ;; only p2 LHS dynamic
  
    (p c3
     =imaginal>
     test-num 3
     value nil
     =goal>
     slot1 =s
     !safe-bind! =v 1
     ==>
     =goal>
     =imaginal>
     color =v
     value t)
  
  (p d3
     =imaginal>
     test-num 3
     =s =y
     value t
     =goal>
     slot1 =s
     ==>
     =goal>
     !output! =y
     =imaginal>
     value busy)
  
  
  #| (p c3+d3
       =imaginal>
         test-num 3
         value nil
       =goal>
          slot1 color
       !safe-bind! =v 1
      ==>
       =goal>
       !output! =v
       =imaginal>
       color =v
       value busy)
  |#

  ;; both dynamic using different slot name binding location
  
    (p c4
     =imaginal>
     test-num 4
     value nil
     =goal>
     slot1 =s
     !safe-bind! =v 1
     ==>
     =goal>
     =imaginal>
     =s =v
     value t)
  
  (p d4
     =imaginal>
     test-num 4
     =s =y
     value t
     =goal>
     slot2 =s
     ==>
     =goal>
     !output! =y
     =imaginal>
     value busy)
  
  
    #| (p c4+d4
       =imaginal>
         test-num 4
         value nil
       =goal>
          slot1 =s
          slot2 =s
       !safe-bind! =v 1
      ==>
       =goal>
       !output! =v
       =imaginal>
       =s =v
       value busy)
  |#

  
  ;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;
  ;; Tests with 4 possible dynamic location
  ;; LHS,RHS = & LHS,RHS =
  
  ;; all 4 dynamic
  
  (p c5
     =imaginal>
     test-num 5
     value nil
     =s nil
     =goal>
     slot1 =s
     !safe-bind! =v 1
     ==>
     =goal>
     =imaginal>
     =s =v
     value t)
  
  (p d5
     =imaginal>
     test-num 5
     =s =y
     value t
     =goal>
     slot2 =s
     ==>
     =goal>
     !output! =y
     =imaginal>
     =s free
     value busy)
  
      #| (p c5+d5
       =imaginal>
         test-num 5
         value nil
         =s nil
       =goal>
          slot1 =s
          slot2 =s
       !safe-bind! =v 1
      ==>
       =goal>
       !output! =v
       =imaginal>
       =s free
       value busy)
  |#

  
  
  ;; p1 LHS static
  
    (p c6
     =imaginal>
     test-num 6
     value nil
     color nil
     =goal>
     slot1 =s
     !safe-bind! =v 1
     ==>
     =goal>
     =imaginal>
     =s =v
     value t)
  
  (p d6
     =imaginal>
     test-num 6
     =s =y
     value t
     =goal>
     slot2 =s
     ==>
     =goal>
     !output! =y
     =imaginal>
     =s free
     value busy)
  
  
  #| (p c6+d6
       =imaginal>
         test-num 6
         value nil
         color nil
       =goal>
          slot1 color
          slot2 color
       !safe-bind! =v 1
      ==>
       =goal>
       !output! =v
       =imaginal>
       color free
       value busy)
  |#

  ;; p1 RHS static
  
  (p c7
     =imaginal>
     test-num 7
     value nil
     =s nil
     =goal>
     slot1 =s
     !safe-bind! =v 1
     ==>
     =goal>
     =imaginal>
     color =v
     value t)
  
  (p d7
     =imaginal>
     test-num 7
     =s =y
     value t
     =goal>
     slot2 =s
     ==>
     =goal>
     !output! =y
     =imaginal>
     =s free
     value busy)
  
  
  #| (p c7+d7
       =imaginal>
         test-num 7
         value nil
         color nil
       =goal>
          slot1 color
          slot2 color
       !safe-bind! =v 1
      ==>
       =goal>
       !output! =v
       =imaginal>
       color free
       value busy)
  |#
  ;; p2 LHS static
  
   (p c8
     =imaginal>
     test-num 8
     value nil
     =s nil
     =goal>
     slot1 =s
     !safe-bind! =v 1
     ==>
     =goal>
     =imaginal>
     =s =v
     value t)
  
  (p d8
     =imaginal>
     test-num 8
     color =y
     value t
     =goal>
     slot2 =s
     ==>
     =goal>
     !output! =y
     =imaginal>
     =s free
     value busy) 
  
    #| (p c8+d8
       =imaginal>
         test-num 8
         value nil
         color nil
       =goal>
          slot1 color
          slot2 color
       !safe-bind! =v 1
      ==>
       =goal>
       !output! =v
       =imaginal>
       color free
       value busy)
  |#
  
  ;; p2 RHS static
  
    (p c9
     =imaginal>
     test-num 9
     value nil
     =s nil
     =goal>
     slot1 =s
     !safe-bind! =v 1
     ==>
     =goal>
     =imaginal>
     =s =v
     value t)
  
  (p d9
     =imaginal>
     test-num 9
     =s =y
     value t
     =goal>
     slot2 =s
     ==>
     =goal>
     !output! =y
     =imaginal>
     color free
     value busy)  
  
      #| (p c9+d9
       =imaginal>
         test-num 9
         value nil
         color nil
       =goal>
          slot1 color
          slot2 color
       !safe-bind! =v 1
      ==>
       =goal>
       !output! =v
       =imaginal>
       color free
       value busy)
  |#
  
  ;; p1 LHS and RHS static
  
  (p c10
     =imaginal>
     test-num 10
     value nil
     color nil
     =goal>
     slot1 =s
     !safe-bind! =v 1
     ==>
     =goal>
     =imaginal>
     color =v
     value t)
  
  (p d10
     =imaginal>
     test-num 10
     =s =y
     value t
     =goal>
     slot2 =s
     ==>
     =goal>
     !output! =y
     =imaginal>
     =s free
     value busy)  
  
        #| (p c10+d10
       =imaginal>
         test-num 10
         value nil
         color nil
       =goal>
          slot1 =s
          slot2 color
       !safe-bind! =v 1
      ==>
       =goal>
       !output! =v
       =imaginal>
       color free
       value busy)
  |#

  
  
  
  ;; p1 lhs and p2 lhs static
  
  (p c11
     =imaginal>
     test-num 11
     value nil
     color nil
     =goal>
     slot1 =s
     !safe-bind! =v 1
     ==>
     =goal>
     =imaginal>
     =s =v
     value t)
  
  (p d11
     =imaginal>
     test-num 11
     color =y
     value t
     =goal>
     slot2 =s
     ==>
     =goal>
     !output! =y
     =imaginal>
     =s free
     value busy)  
  
  #| (p c11+d11
       =imaginal>
         test-num 11
         value nil
         color nil
       =goal>
          slot1 color
          slot2 color
       !safe-bind! =v 1
      ==>
       =goal>
       !output! =v
       =imaginal>
       color free
       value busy)
  |#
 
  ;; p1 lhs and p2 rhs static
  
    (p c12
     =imaginal>
     test-num 12
     value nil
     color nil
     =goal>
     slot1 =s
     !safe-bind! =v 1
     ==>
     =goal>
     =imaginal>
     =s =v
     value t)
  
  (p d12
     =imaginal>
     test-num 12
     =s =y
     value t
     =goal>
     slot2 =s
     ==>
     =goal>
     !output! =y
     =imaginal>
     color free
     value busy)  
  #|
(p c12+d12
   =imaginal>
   test-num 12 
   value nil
   color nil
   =goal>
   slot1 color
   slot2 color
   !safe-bind! =v 1
   ==>
   =goal>
   !output! =v
   =imaginal>
   color free
   value busy)
|#

  
  ;; P1 rhs and p2 lhs static
  
      (p c13
     =imaginal>
     test-num 13
     value nil
     =s nil
     =goal>
     slot1 =s
     !safe-bind! =v 1
     ==>
     =goal>
     =imaginal>
     color =v
     value t)
  
  (p d13
     =imaginal>
     test-num 13
     color =y
     value t
     =goal>
     slot2 =s
     ==>
     =goal>
     !output! =y
     =imaginal>
     =s free
     value busy)  


#|

(p c13+d13
   =imaginal>
   test-num 13
   value nil
   color nil
   =goal>
   slot1 color
   slot2 color
   
   !safe-bind! =v 1
   ==>
   =goal>
   
   !output! =v
   
   =imaginal>
   color free
   value busy
   )
|#
  
  ;; p1 rhs and p2 rhs static
  
  (p c14
     =imaginal>
     test-num 14
     value nil
     =s nil
     =goal>
     slot1 =s
     !safe-bind! =v 1
     ==>
     =goal>
     =imaginal>
     color =v
     value t)
  
  (p d14
     =imaginal>
     test-num 14
     =s =y
     value t
     =goal>
     slot2 =s
     ==>
     =goal>
     !output! =y
     =imaginal>
     color free
     value busy)
  
  
#|

(p c14+d14
   =imaginal>
   test-num 14
   value nil
   color nil
   
   =goal>
   slot1 color
   slot2 color
   
   !safe-bind! =v 1
   ==>
   =goal>
   
   !output! =v
   
   =imaginal>
   color free
   value busy
   )
|#
    ;; p2 lhs and rhs static
  
  (p c15
     =imaginal>
     test-num 15
     value nil
     =s nil
     =goal>
     slot1 =s
     !safe-bind! =v 1
     ==>
     =goal>
     =imaginal>
     =s =v
     value t)
  
  (p d15
     =imaginal>
     test-num 15
     color =y
     value t
     =goal>
     slot2 =s
     ==>
     =goal>
     !output! =y
     =imaginal>
     color free
     value busy)
 
#|

(p c15+d15
   =imaginal>
   test-num 15
   value nil
   color     nil
   
   =goal>
   slot1 color
   slot2 =s
   
   !safe-bind! =v 1
   ==>
   =goal>
   
   !output! =v
   
   =imaginal>
   color      free
   value busy
   )
|# 
  
  ;; only p1 lhs dynamic
  
    (p c16
     =imaginal>
     test-num 16
     value nil
     =s nil
     =goal>
     slot1 =s
     !safe-bind! =v 1
     ==>
     =goal>
     =imaginal>
     color =v
     value t)
  
  (p d16
     =imaginal>
     test-num 16
     color =y
     value t
     =goal>
     slot2 =s
     ==>
     =goal>
     !output! =y
     =imaginal>
     color free
     value busy)
  
 #|

(p c16+d16
   =imaginal>
   test-num 16
   value nil
   color     nil
   
   =goal>
   slot1 color
   slot2 =s2
   
   !safe-bind! =v 1
   ==>
   =goal>
   
   !output! =v
   
   =imaginal>
   color      free
   value busy
   )
|# 
  ;; only p1 rhs dynamic
  
  (p c17
     =imaginal>
     test-num 17
     value nil
     color nil
     =goal>
     slot1 =s
     !safe-bind! =v 1
     ==>
     =goal>
     =imaginal>
     =s =v
     value t)
  
  (p d17
     =imaginal>
     test-num 17
     color =y
     value t
     =goal>
     slot2 =s
     ==>
     =goal>
     !output! =y
     =imaginal>
     color free
     value busy)
 
#|

(p c17+d17
   =imaginal>
   test-num 17
   value nil
   color     nil
   
   =goal>
   slot1 color
   slot2 =s
   
   !safe-bind! =v 1
   ==>
   =goal>
   
   !output! =v
   
   =imaginal>
   color      free
   value busy
   )
|# 
    ;; only p2 lhs dynamic
  
  (p c18
     =imaginal>
     test-num 18
     value nil
     color nil
     =goal>
     slot1 =s
     !safe-bind! =v 1
     ==>
     =goal>
     =imaginal>
     color =v
     value t)
  
  (p d18
     =imaginal>
     test-num 18
     =s =y
     value t
     =goal>
     slot2 =s
     ==>
     =goal>
     !output! =y
     =imaginal>
     color free
     value busy)
 
#|

(p c18+d18
   =imaginal>
   test-num 18
   value nil
   color     nil
   
   =goal>
   slot1 =s 
   slot2 color
   
   !safe-bind! =v 1
   ==>
   =goal>
   
   !output! =v
   
   =imaginal>
   color      free
   value busy
   )
|# 
    ;; only p2 rhs dynamic
  
  (p c19
     =imaginal>
     test-num 19
     value nil
     color nil
     =goal>
     slot1 =s
     !safe-bind! =v 1
     ==>
     =goal>
     =imaginal>
     color =v
     value t)
  
  (p d19
     =imaginal>
     test-num 19
     color =y
     value t
     =goal>
     slot2 =s
     ==>
     =goal>
     !output! =y
     =imaginal>
     =s free
     value busy)

#|

give this one a close look because if the action is variablized it needs to also have color =v

(p c19+d19
   =imaginal>
   test-num 19
   value nil
   color     nil
   
   =goal>
   slot1 =s
   slot2 color
   
   !safe-bind! =v 1
   ==>
   =goal>
   
   !output! =v
   
   =imaginal>
   color      free
   value busy
   )
|#  
  
  ;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;
  ;; Tests with 4 possible dynamic location
  ;; LHS,RHS + & LHS,RHS =  
  
  ;; all 4 dynamic
  
  (p c20
     =imaginal>
     test-num 20
     value nil
     =s nil
     =goal>
     slot1 =s
     !safe-bind! =v 1
     ?imaginal>
     state free
     ==>
     =goal>
     +imaginal>
     test-num 20
     =s =v
     value t)
  
  (p d20
     =imaginal>
     test-num 20
     =s =y
     value t
     =goal>
     slot2 =s
     ==>
     =goal>
     !output! =y
     =imaginal>
     =s free
     value busy)
  

#|

(p c20+d20
   =imaginal>
   test-num 20
   value nil
   =s     nil
   
   =goal>
   slot1 =s
   slot2 =s
   
   !safe-bind! =v 1
        ?imaginal>
     state free
   ==>
   =goal>
   
   !output! =v
   
   +imaginal>
   test-num 20
   =s      free
   value busy
   )
|#

  ;; p1 LHS static
  
    (p c21
     =imaginal>
     test-num 21
     value nil
     color nil
     =goal>
     slot1 =s
     !safe-bind! =v 1
     ?imaginal>
     state free
     ==>
     =goal>
     +imaginal>
     test-num 21
     =s =v
     value t)
  
  (p d21
     =imaginal>
     test-num 21
     =s =y
     value t
     =goal>
     slot2 =s
     ==>
     =goal>
     !output! =y
     =imaginal>
     =s free
     value busy)
  

#|

(p c21+d21
   =imaginal>
   test-num 21
   value nil
   color     nil
   
   =goal>
   slot1 =s 
   slot2 =s
   
   !safe-bind! =v 1
     ?imaginal>
   state free
   ==>
   =goal>
   
   !output! =v
   
   +imaginal>
   test-num 21
   =s      free
   value busy
   )
|#


  ;; p1 RHS static
  
  (p c22
     =imaginal>
     test-num 22
     value nil
     =s nil
     =goal>
     slot1 =s
     !safe-bind! =v 1
     ?imaginal>
     state free
     ==>
     =goal>
     +imaginal>
     test-num 22
     color =v
     value t)
  
  (p d22
     =imaginal>
     test-num 22
     =s =y
     value t
     =goal>
     slot2 =s
     ==>
     =goal>
     !output! =y
     =imaginal>
     =s free
     value busy)
#|

(p c22+d22
   =imaginal>
   test-num 22
   value nil
   =s     nil
   
   =goal>
   slot1 =s
   slot2 color
   
   !safe-bind! =v 1
     ?imaginal>
   state free
   ==>
   =goal>
   
   !output! =v
   
   +imaginal>
   test-num 22
   color      free
   value busy
   )
|#  
  ;; p2 LHS static
  
   (p c23
     =imaginal>
     test-num 23
     value nil
     =s nil
     =goal>
     slot1 =s
     !safe-bind! =v 1
     ?imaginal>
     state free
     ==>
     =goal>
     +imaginal>
     test-num 23
     =s =v
     value t)
  
  (p d23
     =imaginal>
     test-num 23
     color =y
     value t
     =goal>
     slot2 =s
     ==>
     =goal>
     !output! =y
     =imaginal>
     =s free
     value busy) 
  #|

(p c23+d23
   =imaginal>
   test-num 23
   value nil
   color     nil
   
   =goal>
   slot1 color
   slot2 color
   
   !safe-bind! =v 1
     ?imaginal>
   state free
   ==>
   =goal>
   
   !output! =v
   
   +imaginal>
      test-num 23
   color      free
   value busy
   )
|#
  ;; p2 RHS static
  
    (p c24
     =imaginal>
     test-num 24
     value nil
     =s nil
     =goal>
     slot1 =s
     !safe-bind! =v 1
     ?imaginal>
     state free
     ==>
     =goal>
     +imaginal>
     test-num 24
     =s =v
     value t)
  
  (p d24
     =imaginal>
     test-num 24
     =s =y
     value t
     =goal>
     slot2 =s
     ==>
     =goal>
     !output! =y
     =imaginal>
     color free
     value busy)  
  
#|

(p c24+d24
   =imaginal>
   test-num 24
   value nil
   color     nil
   
   =goal>
   slot1 color
   slot2 color
   
   !safe-bind! =v 1
     ?imaginal>
   state free
   ==>
   =goal>
   
   !output! =v
   
   +imaginal>
   test-num 24
   color      free
   value busy
   )
|#
  
  ;; p1 LHS and RHS static
  
  (p c25
     =imaginal>
     test-num 25
     value nil
     color nil
     =goal>
     slot1 =s
     !safe-bind! =v 1
     ?imaginal>
     state free
     ==>
     =goal>
     +imaginal>
     test-num 25
     color =v
     value t)
  
  (p d25
     =imaginal>
     test-num 25
     =s =y
     value t
     =goal>
     slot2 =s
     ==>
     =goal>
     !output! =y
     =imaginal>
     =s free
     value busy)  
  
#|

(p c25+d25
   =imaginal>
   test-num 25
   value nil
   color     nil
   
   =goal>
   slot1 =s
   slot2 color
   
   !safe-bind! =v 1
     ?imaginal>
   state free
   ==>
   =goal>
   
   !output! =v
   
   +imaginal>
      test-num 25

   color      free
   value busy
   )
|#
  ;; p1 lhs and p2 lhs static
  
  (p c26
     =imaginal>
     test-num 26
     value nil
     color nil
     =goal>
     slot1 =s
     !safe-bind! =v 1
     ?imaginal>
     state free
     ==>
     =goal>
     +imaginal>
      test-num 26
    =s =v
     value t)
  
  (p d26
     =imaginal>
     test-num 26
     color =y
     value t
     =goal>
     slot2 =s
     ==>
     =goal>
     !output! =y
     =imaginal>
     =s free
     value busy)  
  
  #|

(p c26+d26
   =imaginal>
   test-num 26
   value nil
   color     nil
   
   =goal>
   slot1 color 
   slot2 color
   
   !safe-bind! =v 1
     ?imaginal>
   state free
   ==>
   =goal>
   
   !output! =v
   
   +imaginal>
      test-num 26

   color      free
   value busy
   )
|#

  ;; p1 lhs and p2 rhs static
  
    (p c27
     =imaginal>
     test-num 27
     value nil
     color nil
     =goal>
     slot1 =s
     !safe-bind! =v 1
     ?imaginal>
     state free
     ==>
     =goal>
     +imaginal>
     test-num 27
     =s =v
     value t)
  
  (p d27
     =imaginal>
     test-num 27
     =s =y
     value t
     =goal>
     slot2 =s
     ==>
     =goal>
     !output! =y
     =imaginal>
     color free
     value busy)  
  
  #|

(p c27+d27
   =imaginal>
   test-num 27
   value nil
   color     nil
   
   =goal>
   slot1 color 
   slot2 color
   
   !safe-bind! =v 1
     ?imaginal>
   state free
   ==>
   =goal>
   
   !output! =v
   
   +imaginal>
      test-num 27

   color      free
   value busy
   )
|#
  ;; P1 rhs and p2 lhs static
  
      (p c28
     =imaginal>
     test-num 28
     value nil
     =s nil
     =goal>
     slot1 =s
     !safe-bind! =v 1
     ?imaginal>
     state free
     ==>
     =goal>
     +imaginal>
     test-num 28
     color =v
     value t)
  
  (p d28
     =imaginal>
     test-num 28
     color =y
     value t
     =goal>
     slot2 =s
     ==>
     =goal>
     !output! =y
     =imaginal>
     =s free
     value busy)  
  
#|

(p c28+d28
   =imaginal>
   test-num 28
   value nil
    =s    nil
   
   =goal>
   slot1 =s
   slot2 color
   
   !safe-bind! =v 1
     ?imaginal>
   state free
   ==>
   =goal>
   
   !output! =v
   
   +imaginal>
   test-num 28
   color      free
   value busy
   )
|#
  ;; p1 rhs and p2 rhs static
  
  (p c29
     =imaginal>
     test-num 29
     value nil
     =s nil
     =goal>
     slot1 =s
     !safe-bind! =v 1
     ?imaginal>
     state free
     ==>
     =goal>
     +imaginal>
     test-num 29
     color =v
     value t)
  
  (p d29
     =imaginal>
     test-num 29
     =s =y
     value t
     =goal>
     slot2 =s
     ==>
     =goal>
     !output! =y
     =imaginal>
     color free
     value busy)
  

#|

(p c29+d29
   =imaginal>
   test-num 29
   value nil
   =s     nil
   
   =goal>
   slot1 =s
   slot2 color
   
   !safe-bind! =v 1
     ?imaginal>
   state free
   ==>
   =goal>
   
   !output! =v
   
   +imaginal>
   test-num 29
   color     free
   value busy
   )
|#
  
    ;; p2 lhs and rhs static
  
  (p c30
     =imaginal>
     test-num 30
     value nil
     =s nil
     =goal>
     slot1 =s
     !safe-bind! =v 1
     ?imaginal>
     state free
     ==>
     =goal>
     +imaginal>
     test-num 30
     =s =v
     value t)
  
  (p d30
     =imaginal>
     test-num 30
     color =y
     value t
     =goal>
     slot2 =s
     ==>
     =goal>
     !output! =y
     =imaginal>
     color free
     value busy)
#|

(p c30+d30
   =imaginal>
   test-num 30
   value nil
    color    nil
   
   =goal>
   slot1 color
   slot2 =s
   
   !safe-bind! =v 1
     ?imaginal>
   state free
   ==>
   =goal>
   
   !output! =v
   
   +imaginal>
   test-num 30
   color      free
   value busy
   )
|#  
  
  ;; only p1 lhs dynamic
  
    (p c31
     =imaginal>
     test-num 31
     value nil
     =s nil
     =goal>
     slot1 =s
     !safe-bind! =v 1
     ?imaginal>
     state free
     ==>
     =goal>
     +imaginal>
     test-num 31
     color =v
     value t)
  
  (p d31
     =imaginal>
     test-num 31
     color =y
     value t
     =goal>
     slot2 =s
     ==>
     =goal>
     !output! =y
     =imaginal>
     color free
     value busy)
  
  
#|

(p c31+d31
   =imaginal>
   test-num 31
   value nil
   =s     nil
   
   =goal>
   slot1 =s
   slot2 =s1
   
   !safe-bind! =v 1
     ?imaginal>
   state free
   ==>
   =goal>
   
   !output! =v
   
   +imaginal>
   test-num 31
   color      free
   value busy
   )
|#
  ;; only p1 rhs dynamic
  
  (p c32
     =imaginal>
     test-num 32
     value nil
     color nil
     =goal>
     slot1 =s
     !safe-bind! =v 1
     ?imaginal>
     state free
     ==>
     =goal>
     +imaginal>
     test-num 32
     =s =v
     value t)
  
  (p d32
     =imaginal>
     test-num 32
     color =y
     value t
     =goal>
     slot2 =s
     ==>
     =goal>
     !output! =y
     =imaginal>
     color free
     value busy)
  
#|

(p c32+d32
   =imaginal>
   test-num 32
   value nil
   color     nil
   
   =goal>
   slot1 color 
   slot2 =s
   
   !safe-bind! =v 1
     ?imaginal>
   state free
   ==>
   =goal>
   
   !output! =v
   
   +imaginal>
   test-num 32
   color      free
   value busy
   )
|#

    ;; only p2 lhs dynamic
  
  (p c33
     =imaginal>
     test-num 33
     value nil
     color nil
     =goal>
     slot1 =s
     !safe-bind! =v 1
     ?imaginal>
     state free
     ==>
     =goal>
     +imaginal>
     test-num 33
     color =v
     value t)
  
  (p d33
     =imaginal>
     test-num 33
     =s =y
     value t
     =goal>
     slot2 =s
     ==>
     =goal>
     !output! =y
     =imaginal>
     color free
     value busy)
  #|

(p c33+d33
   =imaginal>
   test-num 33
   value nil
   color     nil
   
   =goal>
   slot1 =s
   slot2 color
   
   !safe-bind! =v 1
   ==>
     ?imaginal>
   test-num 33
   state free
   =goal>
   
   !output! =v
   
   +imaginal>
    color     free
   value busy
   )
|#
    ;; only p2 rhs dynamic
  
  (p c34
     =imaginal>
     test-num 34
     value nil
     color nil
     =goal>
     slot1 =s
     !safe-bind! =v 1
     ?imaginal>
     state free
     ==>
     =goal>
     +imaginal>
      test-num 34
    color =v
     value t)
  
  (p d34
     =imaginal>
     test-num 34
     color =y
     value t
     =goal>
     slot2 =s
     ==>
     =goal>
     !output! =y
     =imaginal>
     =s free
     value busy)
  
  #|

(p c34+d34
   =imaginal>
   test-num 34
   value nil
   color     nil
   
   =goal>
   slot1 =s
   slot2 color
   
   !safe-bind! =v 1
     ?imaginal>
   state free
   ==>
   =goal>
   
   !output! =v
   
   +imaginal>
   test-num 34
   color      free
   value busy
   )
|#
  ;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;
  ;; Tests with 4 possible dynamic location
  ;; LHS,RHS * & LHS,RHS *  
  
  ;; all 4 dynamic
  
  (p c35
     =imaginal>
     test-num 35
     value nil
     =s nil
     =goal>
     slot1 =s
     !safe-bind! =v 1
     ?imaginal> state free
     ==>
     =goal>
     *imaginal>
     =s =v
     value t)
  
  (p d35
     =imaginal>
     test-num 35
     =s =y
     value t
     =goal>
     slot2 =s
     ?imaginal> state free
     ==>
     =goal>
     !output! =y
     *imaginal>
     =s free
     value busy)
 #|

(p c35+d35
   =imaginal>
   test-num 35
   value nil
    =s    nil
   
   =goal>
   slot1 =s
   slot2 =s
   ?imaginal> state free
  
   !safe-bind! =v 1
   ==>
   =goal>
   
   !output! =v
   
   *imaginal>
    =s     free
   value busy
   )
|# 
  ;; p1 LHS static
  
    (p c36
     =imaginal>
     test-num 36
     value nil
     color nil
     =goal>
     slot1 =s
     !safe-bind! =v 1
     ?imaginal> state free
    ==>
     =goal>
     *imaginal>
     =s =v
     value t)
  
  (p d36
     =imaginal>
     test-num 36
     =s =y
     value t
     =goal>
     slot2 =s
     ?imaginal> state free
     ==>
     =goal>
     !output! =y
     *imaginal>
     =s free
     value busy)
 #|

(p c36+d36
   =imaginal>
   test-num 36
   value nil
   color     nil
   
   =goal>
   slot1 color
   slot2 color
   
   !safe-bind! =v 1
   ==>
   =goal>
   
   !output! =v
   
   *imaginal>
     color    free
   value busy
   )
|# 
  ;; p1 RHS static
  
  (p c37
     =imaginal>
     test-num 37
     value nil
     =s nil
     =goal>
     slot1 =s
     !safe-bind! =v 1
      ?imaginal> state free
    ==>
     =goal>
     *imaginal>
     color =v
     value t)
  
  (p d37
     =imaginal>
     test-num 37
     =s =y
     value t
     =goal>
     slot2 =s
     ?imaginal> state free
     ==>
     =goal>
     !output! =y
     *imaginal>
     =s free
     value busy)
  #|

(p c37+d37
   =imaginal>
   test-num 37
   value nil
    color    nil
   
   =goal>
   slot1 color
   slot2 color
   
   !safe-bind! =v 1
   ==>
   =goal>
   
   !output! =v
   
   *imaginal>
    color     free
   value busy
   )
|#  
  ;; p2 LHS static
  
   (p c38
     =imaginal>
     test-num 38
     value nil
     =s nil
     =goal>
     slot1 =s
     !safe-bind! =v 1
      ?imaginal> state free
    ==>
     =goal>
     *imaginal>
     =s =v
     value t)
  
  (p d38
     =imaginal>
     test-num 38
     color =y
     value t
     =goal>
     slot2 =s
     ?imaginal> state free
     ==>
     =goal>
     !output! =y
     *imaginal>
     =s free
     value busy) 
 
 #|

(p c38+d38
   =imaginal>
   test-num 38
   value nil
    color    nil
   
   =goal>
   slot1 color
   slot2 color
   
   !safe-bind! =v 1
   ==>
   =goal>
   
   !output! =v
   
   *imaginal>
    color     free
   value busy
   )
|#  
  ;; p2 RHS static
  
    (p c39
     =imaginal>
     test-num 39
     value nil
     =s nil
     =goal>
     slot1 =s
     !safe-bind! =v 1
      ?imaginal> state free
    ==>
     =goal>
     *imaginal>
     =s =v
     value t)
  
  (p d39
     =imaginal>
     test-num 39
     =s =y
     value t
     =goal>
     slot2 =s
     ?imaginal> state free
     ==>
     =goal>
     !output! =y
     *imaginal>
     color free
     value busy)  
  
   #|

(p c39+d39
   =imaginal>
   test-num 39
   value nil
   color     nil
   
   =goal>
   slot1 color
   slot2 color
   
   !safe-bind! =v 1
   ==>
   =goal>
   
   !output! =v
   
   *imaginal>
    color     free
   value busy
   )
|# 

  ;; p1 LHS and RHS static
  
  (p c40
     =imaginal>
     test-num 40
     value nil
     color nil
     =goal>
     slot1 =s
     !safe-bind! =v 1
       ?imaginal> state free
   ==>
     =goal>
     *imaginal>
     color =v
     value t)
  
  (p d40
     =imaginal>
     test-num 40
     =s =y
     value t
     =goal>
     slot2 =s
     ?imaginal> state free
     ==>
     =goal>
     !output! =y
     *imaginal>
     =s free
     value busy)  
  
 #|

(p c40+d40
   =imaginal>
   test-num 40
   value nil
    color    nil
   
   =goal>
   slot1 =s
   slot2 color
   
   !safe-bind! =v 1
   ==>
   =goal>
   
   !output! =v
   
   *imaginal>
   color      free
   value busy
   )
|# 
  ;; p1 lhs and p2 lhs static
  
  (p c41
     =imaginal>
     test-num 41
     value nil
     color nil
     =goal>
     slot1 =s
     !safe-bind! =v 1
      ?imaginal> state free
    ==>
     =goal>
     *imaginal>
     =s =v
     value t)
  
  (p d41
     =imaginal>
     test-num 41
     color =y
     value t
     =goal>
     slot2 =s
      ?imaginal> state free
    ==>
     =goal>
     !output! =y
     *imaginal>
     =s free
     value busy)  
  
   #|

(p c41+d41
   =imaginal>
   test-num 41
   value nil
    color    nil
   
   =goal>
   slot1 color
   slot2 color
   
   !safe-bind! =v 1
   ==>
   =goal>
   
   !output! =v
   
   *imaginal>
    color     free
   value busy
   )
|# 

  ;; p1 lhs and p2 rhs static
  
    (p c42
     =imaginal>
     test-num 42
     value nil
     color nil
     =goal>
     slot1 =s
     !safe-bind! =v 1
      ?imaginal> state free
    ==>
     =goal>
     *imaginal>
     =s =v
     value t)
  
  (p d42
     =imaginal>
     test-num 42
     =s =y
     value t
     =goal>
     slot2 =s
      ?imaginal> state free
    ==>
     =goal>
     !output! =y
     *imaginal>
     color free
     value busy)  
  
   #|

(p c42+d42
   =imaginal>
   test-num 42
   value nil
   color     nil
   
   =goal>
   slot1 color
   slot2 color
   
   !safe-bind! =v 1
   ==>
   =goal>
   
   !output! =v
   
   *imaginal>
   color      free
   value busy
   )
|# 
  ;; P1 rhs and p2 lhs static
  
      (p c43
     =imaginal>
     test-num 43
     value nil
     =s nil
     =goal>
     slot1 =s
     !safe-bind! =v 1
     ?imaginal> state free
     ==>
     =goal>
     *imaginal>
     color =v
     value t)
  
  (p d43
     =imaginal>
     test-num 43
     color =y
     value t
     =goal>
     slot2 =s
      ?imaginal> state free
    ==>
     =goal>
     !output! =y
     *imaginal>
     =s free
     value busy)  
  
   #|

(p c43+d43
   =imaginal>
   test-num 43
   value nil
   color     nil
   
   =goal>
   slot1 color
   slot2 color
   
   !safe-bind! =v 1
   ==>
   =goal>
   
   !output! =v
   
   *imaginal>
   color      free
   value busy
   )
|# 


  ;; p1 rhs and p2 rhs static
  
  (p c44
     =imaginal>
     test-num 44
     value nil
     =s nil
     =goal>
     slot1 =s
     !safe-bind! =v 1
      ?imaginal> state free
    ==>
     =goal>
     *imaginal>
     color =v
     value t)
  
  (p d44
     =imaginal>
     test-num 44
     =s =y
     value t
     =goal>
     slot2 =s
      ?imaginal> state free
    ==>
     =goal>
     !output! =y
     *imaginal>
     color free
     value busy)
  
     #|

(p c44+d44
   =imaginal>
   test-num 44
   value nil
    color    nil
   
   =goal>
   slot1 color
   slot2 color
   
   !safe-bind! =v 1
   ==>
   =goal>
   
   !output! =v
   
   *imaginal>
   color      free
   value busy
   )
|# 
    ;; p2 lhs and rhs static
  
  (p c45
     =imaginal>
     test-num 45
     value nil
     =s nil
     =goal>
     slot1 =s
     !safe-bind! =v 1
      ?imaginal> state free
    ==>
     =goal>
     *imaginal>
     =s =v
     value t)
  
  (p d45
     =imaginal>
     test-num 45
     color =y
     value t
     =goal>
     slot2 =s
      ?imaginal> state free
    ==>
     =goal>
     !output! =y
     *imaginal>
     color free
     value busy)
  
     #|

(p c45+d45
   =imaginal>
   test-num 45
   value nil
   color     nil
   
   =goal>
   slot1 color
   slot2 =s
   
   !safe-bind! =v 1
   ==>
   =goal>
   
   !output! =v
   
   *imaginal>
    color     free
   value busy
   )
|# 
  ;; only p1 lhs dynamic
  
    (p c46
     =imaginal>
     test-num 46
     value nil
     =s nil
     =goal>
     slot1 =s
     !safe-bind! =v 1
       ?imaginal> state free
   ==>
     =goal>
     *imaginal>
     color =v
     value t)
  
  (p d46
     =imaginal>
     test-num 46
     color =y
     value t
     =goal>
     slot2 =s
      ?imaginal> state free
    ==>
     =goal>
     !output! =y
     *imaginal>
     color free
     value busy)
  
     #|

(p c46+d46
   =imaginal>
   test-num 46
   value nil
     color   nil
   
   =goal>
   slot1 color
   slot2 =s1
   
   !safe-bind! =v 1
   ==>
   =goal>
   
   !output! =v
   
   *imaginal>
   color      free
   value busy
   )
|# 

  ;; only p1 rhs dynamic
  
  (p c47
     =imaginal>
     test-num 47
     value nil
     color nil
     =goal>
     slot1 =s
     !safe-bind! =v 1
      ?imaginal> state free
    ==>
     =goal>
     *imaginal>
     =s =v
     value t)
  
  (p d47
     =imaginal>
     test-num 47
     color =y
     value t
     =goal>
     slot2 =s
     ?imaginal> state free
     ==>
     =goal>
     !output! =y
     *imaginal>
     color free
     value busy)
     #|

(p c47+d47
   =imaginal>
   test-num 47
   value nil
   color     nil
   
   =goal>
   slot1 color 
   slot2 =s
   
   !safe-bind! =v 1
   ==>
   =goal>
   
   !output! =v
   
   *imaginal>
   color      free
   value busy
   )
|# 
    ;; only p2 lhs dynamic
  
  (p c48
     =imaginal>
     test-num 48
     value nil
     color nil
     =goal>
     slot1 =s
     !safe-bind! =v 1
       ?imaginal> state free
   ==>
     =goal>
     *imaginal>
     color =v
     value t)
  
  (p d48
     =imaginal>
     test-num 48
     =s =y
     value t
     =goal>
     slot2 =s
      ?imaginal> state free
    ==>
     =goal>
     !output! =y
     *imaginal>
     color free
     value busy)
     #|

(p c48+d48
   =imaginal>
   test-num 48
   value nil
   color     nil
   
   =goal>
   slot1 =s
   slot2 color
   
   !safe-bind! =v 1
   ==>
   =goal>
   
   !output! =v
   
   *imaginal>
   color      free
   value busy
   )
|# 
    ;; only p2 rhs dynamic
  
  (p c49
     =imaginal>
     test-num 49
     value nil
     color nil
     =goal>
     slot1 =s
     !safe-bind! =v 1
      ?imaginal> state free
    ==>
     =goal>
     *imaginal>
     color =v
     value t)
  
  (p d49
     =imaginal>
     test-num 49
     color =y
     value t
     =goal>
     slot2 =s
     ?imaginal> state free
     ==>
     =goal>
     !output! =y
     *imaginal>
     =s free
     value busy)
  
     #|

(p c49+d49
   =imaginal>
   test-num 49
   value nil
   color     nil
   
   =goal>
   slot1 =s 
   slot2 color
   
   !safe-bind! =v 1
   ==>
   =goal>
   
   !output! =v
   
   *imaginal>
   color  free
   value busy
   )
|# 
   ;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;
  ;; Tests with 5 possible dynamic location
  ;; LHS,RHS =,+ & LHS,RHS =  
  
  ;; all 5 dynamic
  
  (p c50
     =imaginal>
     test-num 50
     value nil
     =s nil
     =goal>
     slot1 =s
     !safe-bind! =v 1
     ?imaginal>
     state free
     ==>
     =goal>
     =imaginal>
     =s =v
     +imaginal>
     test-num 50
     =s =v
     value t)
  
  (p d50
     =imaginal>
     test-num 50
     =s =y
     value t
     =goal>
     slot2 =s
     ?imaginal> state free
     ==>
     =goal>
     !output! =y
     =imaginal>
     =s free
     value busy)
 

    #|

(p c50+d50
   =imaginal>
   test-num 50
   value nil
    =s    nil
   
   =goal>
   slot1  =s
   slot2 =s
   
   !safe-bind! =v 1
     ?imaginal>
   state free
   ==>
   =goal>
   
   !output! =v
   +imaginal>
   test-num 50
   value busy
   =s   free
   =imaginal>
   =s  =v
   )
|# 


 
  ;; p1 LHS static
  
  (p c51
     =imaginal>
     test-num 51
     value nil
     color nil
     =goal>
     slot1 =s
     !safe-bind! =v 1
     ?imaginal>
     state free
     ==>
     =goal>
     =imaginal>
     =s =v
     +imaginal>
     test-num 51
     =s =v
     value t)
  
  (p d51
     =imaginal>
     test-num 51
     =s =y
     value t
     =goal>
     slot2 =s
     ?imaginal> state free
     ==>
     =goal>
     !output! =y
     =imaginal>
     =s free
     value busy)
  
    #|

(p c51+d51
   =imaginal>
   test-num 51
   value nil
    color    nil
   
   =goal>
   slot1  =s
   slot2 =s
   
   !safe-bind! =v 1
     ?imaginal>
   state free
   ==>
   =goal>
   
   !output! =v
   +imaginal>
    test-num 51
  value busy
    =s  free
   =imaginal>
   =s  =v
   )
|# 


  ;; p1 RHS * static
  
    (p c52
     =imaginal>
     test-num 52
     value nil
     =s nil
     =goal>
     slot1 =s
     !safe-bind! =v 1
     ?imaginal>
     state free
     ==>
     =goal>
     =imaginal>
     color =v
     +imaginal>
     test-num 52
     =s =v
     value t)
  
  (p d52
     =imaginal>
     test-num 52
     =s =y
     value t
     =goal>
     slot2 =s
     ?imaginal> state free
     ==>
     =goal>
     !output! =y
     =imaginal>
     =s free
     value busy)


    #|

(p c52+d52
   =imaginal>
   test-num 52
   value nil
    =s    nil
   
   =goal>
   slot1  =s
   slot2 =s 
   
   !safe-bind! =v 1
     ?imaginal>
   state free
   ==>
   =goal>
   
   !output! =v
   +imaginal>
   test-num 52
   value busy
    =s  free
   =imaginal>
    color =v
   )
|#   
    ;; p1 RHS + static
  
    (p c53
     =imaginal>
     test-num 53
     value nil
     =s nil
     =goal>
     slot1 =s
     !safe-bind! =v 1
     ?imaginal>
     state free
     ==>
     =goal>
     =imaginal>
     =s =v
     +imaginal>
     test-num 53
     color =v
     value t)
  
  (p d53
     =imaginal>
     test-num 53
     =s =y
     value t
     =goal>
     slot2 =s
      ?imaginal> state free
    ==>
     =goal>
     !output! =y
     =imaginal>
     =s free
     value busy)
 
    #|

(p c53+d53
   =imaginal>
   test-num 53
   value nil
    =s    nil
   
   =goal>
   slot1  =s
   slot2 color
   
   !safe-bind! =v 1
     ?imaginal>
   state free
   ==>
   =goal>
   
   !output! =v
   +imaginal>
   test-num 53
   value busy
    color  free
   =imaginal>
   =s  =v
   )
|#  
  ;; p2 LHS static
  
    (p c54
     =imaginal>
     test-num 54
     value nil
     =s nil
     =goal>
     slot1 =s
     !safe-bind! =v 1
     ?imaginal>
     state free
     ==>
     =goal>
     =imaginal>
     =s =v
     +imaginal>
     test-num 54
     =s =v
     value t)
  
  (p d54
     =imaginal>
     test-num 54
     color =y
     value t
     =goal>
     slot2 =s
       ?imaginal> state free
   ==>
     =goal>
     !output! =y
     =imaginal>
     =s free
     value busy)  
  
    #|

(p c54+d54
   =imaginal>
   test-num 54
   value nil
    color    nil
   
   =goal>
   slot1  color
   slot2 color
   
   !safe-bind! =v 1
     ?imaginal>
   state free
   ==>
   =goal>
   
   !output! =v
   +imaginal>
    test-num 54
  value busy
    color  free
   =imaginal>
   color  =v
   )
|# 
  
  ;; p2 RHS static
  
      (p c55
     =imaginal>
     test-num 55
     value nil
     =s nil
     =goal>
     slot1 =s
     !safe-bind! =v 1
     ?imaginal>
     state free
     ==>
     =goal>
     =imaginal>
     =s =v
     +imaginal>
     test-num 55
     =s =v
     value t)
  
  (p d55
     =imaginal>
     test-num 55
     =s =y
     value t
     =goal>
     slot2 =s
       ?imaginal> state free
   ==>
     =goal>
     !output! =y
     =imaginal>
     color free
     value busy)  
  
      #|

(p c55+d55
   =imaginal>
   test-num 55
   value nil
   color     nil
   
   =goal>
   slot1  color
   slot2 color
   
   !safe-bind! =v 1
     ?imaginal>
   state free
   ==>
   =goal>
   
   !output! =v
   +imaginal>
    test-num 55
  value busy
   color   free
   =imaginal>
   color  =v
   )
|# 
  ;; 2 static
  
  ;; p1 LHS and RHS * static
  
      (p c56
     =imaginal>
     test-num 56
     value nil
     color nil
     =goal>
     slot1 =s
     !safe-bind! =v 1
     ?imaginal>
     state free
     ==>
     =goal>
     =imaginal>
     color =v
     +imaginal>
     test-num 56
     =s =v
     value t)
  
  (p d56
     =imaginal>
     test-num 56
     =s =y
     value t
     =goal>
     slot2 =s
     ?imaginal> state free
     ==>
     =goal>
     !output! =y
     =imaginal>
     =s free
     value busy)
   
  #|

(p c56+d56
   =imaginal>
   test-num 56
   value nil
   color     nil
   
   =goal>
   slot1  =s
   slot2 =s
   
   !safe-bind! =v 1
     ?imaginal>
   state free
   ==>
   =goal>
   
   !output! =v
   +imaginal>
    test-num 56
  value busy
    =s  free
   =imaginal>
    color =v
   )
|#  
  
    ;; p1 LHS and RHS + static
  
      (p c57
     =imaginal>
     test-num 57
     value nil
     color nil
     =goal>
     slot1 =s
     !safe-bind! =v 1
     ?imaginal>
     state free
     ==>
     =goal>
     =imaginal>
     =s =v
     +imaginal>
     test-num 57
     color =v
     value t)
  
  (p d57
     =imaginal>
     test-num 57
     =s =y
     value t
     =goal>
     slot2 =s
      ?imaginal> state free
    ==>
     =goal>
     !output! =y
     =imaginal>
     =s free
     value busy)
    #|

(p c57+d57
   =imaginal>
   test-num 57
   value nil
    color    nil
   
   =goal>
   slot1  =s
   slot2 color
   
   !safe-bind! =v 1
     ?imaginal>
   state free
   ==>
   =goal>
   
   !output! =v
   +imaginal>
    test-num 57
  value busy
    color  free
   =imaginal>
    =s =v
   )
|# 
  ;; p1 lhs and p2 lhs static
  
      (p c58
     =imaginal>
     test-num 58
     value nil
     color nil
     =goal>
     slot1 =s
     !safe-bind! =v 1
     ?imaginal>
     state free
     ==>
     =goal>
     =imaginal>
     =s =v
     +imaginal>
     test-num 58
     =s =v
     value t)
  
  (p d58
     =imaginal>
     test-num 58
     color =y
     value t
     =goal>
     slot2 =s
      ?imaginal> state free
    ==>
     =goal>
     !output! =y
     =imaginal>
     =s free
     value busy) 
  #|
  (p c58+d58
   =imaginal>
   test-num 58
   value nil
    color    nil
   
   =goal>
   slot1  color
   slot2 color
   
   !safe-bind! =v 1
     ?imaginal>
     state free
     ==>
   =goal>
   
   !output! =v
   +imaginal>
    test-num 58
  value busy
   color  free
   =imaginal>
   color     =v
   )
|#
  ;; p1 lhs and p2 rhs static
  
       (p c59
     =imaginal>
     test-num 59
     value nil
     color nil
     =goal>
     slot1 =s
     !safe-bind! =v 1
     ?imaginal>
     state free
     ==>
     =goal>
     =imaginal>
     =s =v
     +imaginal>
     test-num 59
     =s =v
     value t)
  
  (p d59
     =imaginal>
     test-num 59
     =s =y
     value t
     =goal>
     slot2 =s
      ?imaginal> state free
    ==>
     =goal>
     !output! =y
     =imaginal>
     color free
     value busy)   
  #|
(p c59+d59
   =imaginal>
   test-num 59
   value nil
  color      nil
   
   =goal>
   slot1  color
   slot2 color
   
   !safe-bind! =v 1
     ?imaginal>
   state free
   ==>
   =goal>
   
   !output! =v
   +imaginal>
    test-num 59
  value busy
    color  free
   =imaginal>
    color  =v
   )
|#
   ;; P1 rhs *,+ static
  
        (p c60
     =imaginal>
     test-num 60
     value nil
     =s nil
     =goal>
     slot1 =s
     !safe-bind! =v 1
     ?imaginal>
     state free
     ==>
     =goal>
     =imaginal>
     color =v
     +imaginal>
     test-num 60
     color =v
     value t)
  
  (p d60
     =imaginal>
     test-num 60
     =s =y
     value t
     =goal>
     slot2 =s
      ?imaginal> state free
    ==>
     =goal>
     !output! =y
     =imaginal>
     =s free
     value busy)   
#|  
  (p c60+d60
   =imaginal>
   test-num 60
   value nil
     =s   nil
   
   =goal>
   slot1  =s
   slot2 color
   
   !safe-bind! =v 1
     ?imaginal>
     state free
     ==>
   =goal>
   
   !output! =v
   +imaginal>
   test-num 60
   value busy
    color  free
   =imaginal>
    color =v
     )
|#

  ;; P1 rhs * and p2 lhs static
  
          (p c61
     =imaginal>
     test-num 61
     value nil
     =s nil
     =goal>
     slot1 =s
     !safe-bind! =v 1
     ?imaginal>
     state free
     ==>
     =goal>
     =imaginal>
     color =v
     +imaginal>
     test-num 61
     =s =v
     value t)
  
  (p d61
     =imaginal>
     test-num 61
     color =y
     value t
     =goal>
     slot2 =s
       ?imaginal> state free
   ==>
     =goal>
     !output! =y
     =imaginal>
     =s free
     value busy)   
#|
(p c61+d61
   =imaginal>
   test-num 61
   value nil
    color    nil
   
   =goal>
   slot1 color 
   slot2 color
   
   !safe-bind! =v 1
     ?imaginal>
   state free
   ==>
   =goal>
   
   !output! =v
   +imaginal>
   test-num 61
   value busy
    color  free
   =imaginal>
   color =v
   )

|#
  ;; p1 rhs * and p2 rhs static
  
         (p c62
     =imaginal>
     test-num 62
     value nil
     =s nil
     =goal>
     slot1 =s
     !safe-bind! =v 1
     ?imaginal>
     state free
     ==>
     =goal>
     =imaginal>
     color =v
     +imaginal>
     test-num 62
     =s =v
     value t)
  
  (p d62
     =imaginal>
     test-num 62
     =s =y
     value t
     =goal>
     slot2 =s
       ?imaginal> state free
   ==>
     =goal>
     !output! =y
     =imaginal>
     color free
     value busy)   
  
#|
(p c62+d62
   =imaginal>
   test-num 62
   value nil
    color    nil
   
   =goal>
   slot1  color
   slot2 color
   
   !safe-bind! =v 1
     ?imaginal>
   state free
   ==>
   =goal>
   
   !output! =v
   +imaginal>
    test-num 62
  value busy
    color  free
   =imaginal>
    color =v
   )

|#
  


    ;; P1 rhs + and p2 lhs static
  
          (p c63
     =imaginal>
     test-num 63
     value nil
     =s nil
     =goal>
     slot1 =s
     !safe-bind! =v 1
     ?imaginal>
     state free
     ==>
     =goal>
     =imaginal>
     =s =v
     +imaginal>
     test-num 63
     color =v
     value t)
  
  (p d63
     =imaginal>
     test-num 63
     color =y
     value t
     =goal>
     slot2 =s
       ?imaginal> state free
   ==>
     =goal>
     !output! =y
     =imaginal>
     =s free
     value busy)   
  

#|
(p c63+d63
   =imaginal>
   test-num 63
   value nil
   =s     nil
   
   =goal>
   slot1  =s
   slot2 color
   
   !safe-bind! =v 1
     ?imaginal>
   state free
   ==>
   =goal>
   
   !output! =v
   +imaginal>
    test-num 63
  value busy
   color   free
   =imaginal>
   =s  =v
   )

|#

  ;; p1 rhs + and p2 rhs static
  
         (p c64
     =imaginal>
     test-num 64
     value nil
     =s nil
     =goal>
     slot1 =s
     !safe-bind! =v 1
     ?imaginal>
     state free
     ==>
     =goal>
     =imaginal>
     =s =v
     +imaginal>
     test-num 64
     color =v
     value t)
  
  (p d64
     =imaginal>
     test-num 64
     =s =y
     value t
     =goal>
     slot2 =s
       ?imaginal> state free
   ==>
     =goal>
     !output! =y
     =imaginal>
     color free
     value busy)   
  
 #|
(p c64+d64
   =imaginal>
   test-num 64
   value nil
      =s  nil
   
   =goal>
   slot1 =s  
   slot2 color
   
   !safe-bind! =v 1
     ?imaginal>
   state free
   ==>
   =goal>
   
   !output! =v
   +imaginal>
    test-num 64
  value busy
    color  free
   =imaginal>
    =s =v
   )

|#
 
    ;; p2 lhs and rhs static
  
          (p c65
     =imaginal>
     test-num 65
     value nil
     =s nil
     =goal>
     slot1 =s
     !safe-bind! =v 1
     ?imaginal>
     state free
     ==>
     =goal>
     =imaginal>
     =s =v
     +imaginal>
     test-num 65
     =s =v
     value t)
  
  (p d65
     =imaginal>
     test-num 65
     color =y
     value t
     =goal>
     slot2 =s
        ?imaginal> state free
  ==>
     =goal>
     !output! =y
     =imaginal>
     color free
     value busy)   
    
  #|
(p c65+d65
   =imaginal>
   test-num 65
   value nil
    color    nil
   
   =goal>
   slot1  color
   slot2 =s
   
   !safe-bind! =v 1
     ?imaginal>
   state free
   ==>
   =goal>
   
   !output! =v
   +imaginal>
    test-num 65
  value busy
    color  free
   =imaginal>
   color     =v
   )

|#

  
    ;; 3 static
  
  
    ;; p1 LHS and RHS * + static
  
      (p c66
     =imaginal>
     test-num 66
     value nil
     color nil
     =goal>
     slot1 =s
     !safe-bind! =v 1
          ?imaginal>
     state free
==>
     =goal>
     =imaginal>
     color =v
     +imaginal>
     test-num 66
     color =v
     value t)
  
  (p d66
     =imaginal>
     test-num 66
     =s =y
     value t
     =goal>
     slot2 =s
        ?imaginal> state free
  ==>
     =goal>
     !output! =y
     =imaginal>
     =s free
     value busy)
 
#|
(p c66+d66
   =imaginal>
   test-num 66
   value nil
    color   nil
   
   =goal>
   slot1  =s
   slot2 color
   
   !safe-bind! =v 1
     ?imaginal>
   state free
   ==>
   =goal>
   
   !output! =v
   +imaginal>
    test-num 66
  value busy
    color  free
   =imaginal>
   color  =v
   )

|#
 
  
    ;; p1 LHS RHS *  p2 lhs static
  
      (p c67
     =imaginal>
     test-num 67
     value nil
     color nil
     =goal>
     slot1 =s
     !safe-bind! =v 1
          ?imaginal>
     state free
==>
     =goal>
     =imaginal>
     color =v
     +imaginal>
     test-num 67
     =s =v
     value t)
  
  (p d67
     =imaginal>
     test-num 67
     color =y
     value t
     =goal>
     slot2 =s
       ?imaginal> state free
   ==>
     =goal>
     !output! =y
     =imaginal>
     =s free
     value busy)
  
#|

could there be some trickery in this one?

(p c67+d67
   =imaginal>
   test-num 67
   value nil
    color    nil
   
   =goal>
   slot1 color  
   slot2 color
   
   !safe-bind! =v 1
     ?imaginal>
   state free
   ==>
   =goal>
   
   !output! =v
   +imaginal>
    test-num 67
  value busy
   color   free
   =imaginal>
    color =v
   )

|#

  ;; p1 lhs rhs * and p2 rhs static
  
      (p c68
     =imaginal>
     test-num 68
     value nil
     color nil
     =goal>
     slot1 =s
     !safe-bind! =v 1
          ?imaginal>
     state free
==>
     =goal>
     =imaginal>
     color =v
     +imaginal>
     test-num 68
     =s =v
     value t)
  
  (p d68
     =imaginal>
     test-num 68
     =s =y
     value t
     =goal>
     slot2 =s
        ?imaginal> state free
  ==>
     =goal>
     !output! =y
     =imaginal>
     color free
     value busy) 
  
  #|
(p c68+d68
   =imaginal>
   test-num 68
   value nil
   color     nil
   
   =goal>
   slot1  color
   slot2 color
   
   !safe-bind! =v 1
     ?imaginal>
   state free
   ==>
   =goal>
   
   !output! =v
   +imaginal>
    test-num 68
  value busy
   color   free
   =imaginal>
    color =v
   )

|#

  ;; p1 lhs rhs + and p2 lhs static
  
       (p c69
     =imaginal>
     test-num 69
     value nil
     color nil
     =goal>
     slot1 =s
     !safe-bind! =v 1
     ?imaginal>
     state free
     ==>
     =goal>
     =imaginal>
     =s =v
     +imaginal>
     test-num 69
     color =v
     value t)
  
  (p d69
     =imaginal>
     test-num 69
     color =y
     value t
     =goal>
     slot2 =s
        ?imaginal> state free
  ==>
     =goal>
     !output! =y
     =imaginal>
     =s free
     value busy)   
  
 #|
(p c69+d69
   =imaginal>
   test-num 69
   value nil
   color     nil
   
   =goal>
   slot1  =s
   slot2 color
   
   !safe-bind! =v 1
     ?imaginal>
   state free
   ==>
   =goal>
   
   !output! =v
   +imaginal>
    test-num 69
  value busy
    color  free
   =imaginal>
   =s  =v
   )

|#
 
   ;; P1 lhs rhs + and p2 rhs static
  
        (p c70
     =imaginal>
     test-num 70
     value nil
     color nil
     =goal>
     slot1 =s
     !safe-bind! =v 1
     ?imaginal>
     state free
     ==>
     =goal>
     =imaginal>
     =s =v
     +imaginal>
     test-num 70
     color =v
     value t)
  
  (p d70
     =imaginal>
     test-num 70
     =s =y
     value t
     =goal>
     slot2 =s
        ?imaginal> state free
  ==>
     =goal>
     !output! =y
     =imaginal>
     color free
     value busy)   

  #|
(p c70+d70
   =imaginal>
   test-num 70
   value nil
    color    nil
   
   =goal>
   slot1  =s
   slot2 color
   
   !safe-bind! =v 1
     ?imaginal>
   state free
   ==>
   =goal>
   
   !output! =v
   +imaginal>
     test-num 70
 value busy
    color  free
   =imaginal>
    =s =v
   )

|#

  
  ;; P1 lhs and p2 lhs rhs static
  
          (p c71
     =imaginal>
     test-num 71
     value nil
     color nil
     =goal>
     slot1 =s
     !safe-bind! =v 1
     ?imaginal>
     state free
     ==>
     =goal>
     =imaginal>
     =s =v
     +imaginal>
     test-num 71
     =s =v
     value t)
  
  (p d71
     =imaginal>
     test-num 71
     color =y
     value t
     =goal>
     slot2 =s
        ?imaginal> state free
  ==>
     =goal>
     !output! =y
     =imaginal>
     color free
     value busy)   
  
#|
(p c71+d71
   =imaginal>
   test-num 71
   value nil
    color    nil
   
   =goal>
   slot1  color
   slot2 =s
   
   !safe-bind! =v 1
     ?imaginal>
   state free
   ==>
   =goal>
   
   !output! =v
   +imaginal>
      test-num 71
  value busy
   color   free
   =imaginal>
   color  =v
   )

|#

  ;; p1 rhs * + and p2 lhs static
  
         (p c72
     =imaginal>
     test-num 72
     value nil
     =s nil
     =goal>
     slot1 =s
     !safe-bind! =v 1
     ?imaginal>
     state free
     ==>
     =goal>
     =imaginal>
     color =v
     +imaginal>
      test-num 72
    color =v
     value t)
  
  (p d72
     =imaginal>
     test-num 72
     color =y
     value t
     =goal>
     slot2 =s
        ?imaginal> state free
  ==>
     =goal>
     !output! =y
     =imaginal>
     =s free
     value busy)   
 
#|
(p c72+d72
   =imaginal>
   test-num 72
   value nil
    =s    nil
   
   =goal>
   slot1  =s
   slot2 color
   
   !safe-bind! =v 1
     ?imaginal>
   state free
   ==>
   =goal>
   
   !output! =v
   +imaginal>
     test-num 72
   value busy
   color   free
   =imaginal>
   color  =v
   )

|#
 
  
    ;; P1 rhs * + and p2 rhs static
  
          (p c73
     =imaginal>
     test-num 73
     value nil
     =s nil
     =goal>
     slot1 =s
     !safe-bind! =v 1
     ?imaginal>
     state free
     ==>
     =goal>
     =imaginal>
     color =v
     +imaginal>
     test-num 73
     color =v
     value t)
  
  (p d73
     =imaginal>
     test-num 73
     =s =y
     value t
     =goal>
     slot2 =s
        ?imaginal> state free
  ==>
     =goal>
     !output! =y
     =imaginal>
     color free
     value busy)   
  
#|
(p c73+d73
   =imaginal>
   test-num 73
   value nil
      =s  nil
   
   =goal>
   slot1 =s  
   slot2 color
   
   !safe-bind! =v 1
     ?imaginal>
   state free
   ==>
   =goal>
   
   !output! =v
   +imaginal>
     test-num 73
   value busy
     color free
   =imaginal>
    color =v
   )

|#



  ;; p1 rhs * and p2 lhs rhs static
  
         (p c74
     =imaginal>
     test-num 74
     value nil
     =s nil
     =goal>
     slot1 =s
     !safe-bind! =v 1
         ?imaginal>
     state free
 ==>
     =goal>
     =imaginal>
     color =v
     +imaginal>
     test-num 74
     =s =v
     value t)
  
  (p d74
     =imaginal>
     test-num 74
     color =y
     value t
     =goal>
     slot2 =s
        ?imaginal> state free
  ==>
     =goal>
     !output! =y
     =imaginal>
     color free
     value busy)   
  

#|
(p c74+d74
   =imaginal>
   test-num 74
   value nil
    color    nil
   
   =goal>
   slot1 color 
   slot2 =s
   
   !safe-bind! =v 1
     ?imaginal>
   state free
   ==>
   =goal>
   
   !output! =v
   +imaginal>
      test-num 74
  value busy
    color  free
   =imaginal>
   color  =v
   )

|#

    
    ;; p1 rhs + and p2 lhs and rhs static
  
          (p c75
     =imaginal>
     test-num 75
     value nil
     =s nil
     =goal>
     slot1 =s
     !safe-bind! =v 1
     ?imaginal>
     state free
     ==>
     =goal>
     =imaginal>
     =s =v
     +imaginal>
     test-num 75
     color =v
     value t)
  
  (p d75
     =imaginal>
     test-num 75
     color =y
     value t
     =goal>
     slot2 =s
        ?imaginal> state free
  ==>
     =goal>
     !output! =y
     =imaginal>
     color free
     value busy)   
    
  #|
(p c75+d75
   =imaginal>
   test-num 75
   value nil
    =s    nil
   
   =goal>
   slot1  =s
   slot2 =s2
   
   !safe-bind! =v 1
     ?imaginal>
   state free
   ==>
   =goal>
   
   !output! =v
   +imaginal>
     test-num 75
   value busy
   color   free
   =imaginal>
    =s =v
   )

|#

  
  
  ;; 4 static
  
   
  
  ;; only p1 lhs dynamic
  
  (p c76
     =imaginal>
     test-num 76
     value nil
     =s nil
     =goal>
     slot1 =s
     !safe-bind! =v 1
     ?imaginal>
     state free
     ==>
     =goal>
     =imaginal>
     color =v
     +imaginal>
     test-num 76
     color =v
     value t)
  
  (p d76
     =imaginal>
     test-num 76
     color =y
     value t
     =goal>
     slot2 =s
        ?imaginal> state free
  ==>
     =goal>
     !output! =y
     =imaginal>
     color free
     value busy)   
 
#|
(p c76+d76
   =imaginal>
   test-num 76
   value nil
    =s    nil
   
   =goal>
   slot1  =s
   slot2 =s2
   
   !safe-bind! =v 1
     ?imaginal>
   state free
   ==>
   =goal>
   
   !output! =v
   +imaginal>
    test-num 76
    value busy
    color  free
   =imaginal>
   color  =v
   )

|#

     
  ;; only p1 rhs * dynamic
  
   (p c77
     =imaginal>
     test-num 77
     value nil
     color nil
     =goal>
     slot1 =s
     !safe-bind! =v 1
     ?imaginal>
     state free
     ==>
     =goal>
     =imaginal>
     =s =v
     +imaginal>
     test-num 77
     color =v
     value t)
  
  (p d77
     =imaginal>
     test-num 77
     color =y
     value t
     =goal>
     slot2 =s
        ?imaginal> state free
  ==>
     =goal>
     !output! =y
     =imaginal>
     color free
     value busy)   
  

#|
(p c77+d77
   =imaginal>
   test-num 77
   value nil
    color    nil
   
   =goal>
   slot1  =s
   slot2 =s2
   
   !safe-bind! =v 1
     ?imaginal>
   state free
   ==>
   =goal>
   
   !output! =v
   +imaginal>
    test-num 77
  value busy
     color free
   =imaginal>
   =s  =v
   )

|#

    ;; only p1 rhs + dynamic
  
   (p c78
     =imaginal>
     test-num 78
     value nil
     color nil
     =goal>
     slot1 =s
     !safe-bind! =v 1
     ?imaginal>
     state free
     ==>
     =goal>
     =imaginal>
     color =v
     +imaginal>
     test-num 78
     =s =v
     value t)
  
  (p d78
     =imaginal>
     test-num 78
     color =y
     value t
     =goal>
     slot2 =s
        ?imaginal> state free
  ==>
     =goal>
     !output! =y
     =imaginal>
     color free
     value busy)   
  #|
(p c78+d78
   =imaginal>
   test-num 78
   value nil
    color    nil
   
   =goal>
   slot1  color
   slot2 =s
   
   !safe-bind! =v 1
     ?imaginal>
   state free
   ==>
   =goal>
   
   !output! =v
   +imaginal>
   test-num 78
   value busy
    color  free
   =imaginal>
    color =v
   )

|#

  
  ;; only p2 lhs dynamic
  
    (p c79
     =imaginal>
     test-num 79
     value nil
     color nil
     =goal>
     slot1 =s
     !safe-bind! =v 1
     ?imaginal>
     state free
     ==>
     =goal>
     =imaginal>
     color =v
     +imaginal>
     test-num 79
     color =v
     value t)
  
  (p d79
     =imaginal>
     test-num 79
     =s =y
     value t
     =goal>
     slot2 =s
        ?imaginal> state free
  ==>
     =goal>
     !output! =y
     =imaginal>
     color free
     value busy)   
   

#|
(p c79+d79
   =imaginal>
   test-num 79
   value nil
   color     nil
   
   =goal>
   slot1 =s  
   slot2 color
   
   !safe-bind! =v 1
     ?imaginal>
   state free
   ==>
   =goal>
   
   !output! =v
   +imaginal>
   test-num 79
   value busy
    color  free
   =imaginal>
    color =v
   )

|#

 
    ;; only p2 rhs dynamic
  
    (p c80
     =imaginal>
     test-num 80
     value nil
     color nil
     =goal>
     slot1 =s
     !safe-bind! =v 1
     ?imaginal>
     state free
     ==>
     =goal>
     =imaginal>
     color =v
     +imaginal>
      test-num 80
    color =v
     value t)
  
  (p d80
     =imaginal>
     test-num 80
     color =y
     value t
     =goal>
     slot2 =s
        ?imaginal> state free
  ==>
     =goal>
     !output! =y
     =imaginal>
     =s free
     value busy)   
  
  #|
(p c80+d80
   =imaginal>
   test-num 80
   value nil
     color   nil
   
   =goal>
   slot1  =s
   slot2 color
   
   !safe-bind! =v 1
     ?imaginal>
   state free
   ==>
   =goal>
   
   !output! =v
   +imaginal>
   test-num 80
   value busy
    color  free
   =imaginal>
    color =v
   )

|#

  
  
  
  ;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;
  ;; Tests with 5 possible dynamic location
  ;; LHS,RHS = & LHS,RHS = +  
  
  ;; all 5 dynamic
  
  (p c81
     =imaginal>
     test-num 81
     value nil
     =s nil
     =goal>
     slot1 =s
     !safe-bind! =v 1
     ==>
     =goal>
     =imaginal>
     =s =v
     value t)
  
  (p d81
     =imaginal>
     test-num 81
     =s =y
     value t
     =goal>
     slot2 =s
       !safe-bind! =v 1
     ?imaginal>
     state free
   ==>
     =goal>
     !output! =y
     =imaginal>
     =s free
     +imaginal>
     =s =v
     value busy)
  

#|
(p c81+d81
   =imaginal>
   test-num 81
   value nil
    =s    nil
   
   =goal>
   slot1  =s
   slot2 =s
   
   !safe-bind! =v 1
     ?imaginal>
   state free
   ==>
   =goal>
   
   !output! =v
   +imaginal>
   value busy
    =s  =v
   =imaginal>
     =s free
    value t
   )

|#



  ;; p1 LHS static
  
    (p c82
     =imaginal>
     test-num 82
     value nil
     color nil
     =goal>
     slot1 =s
     !safe-bind! =v 1
     ==>
     =goal>
     =imaginal>
     =s =v
     value t)
  
  (p d82
     =imaginal>
     test-num 82
     =s =y
     value t
     =goal>
     slot2 =s
        !safe-bind! =v 1
     ?imaginal>
     state free
  ==>
     =goal>
     !output! =y
     =imaginal>
     =s free
     +imaginal>
     =s =v
     value busy)
  
#|
(p c82+d82
   =imaginal>
   test-num 82
   value nil
   color     nil
   
   =goal>
   slot1  color
   slot2 color
   
   !safe-bind! =v 1
     ?imaginal>
   state free
   ==>
   =goal>
   
   !output! =v
   +imaginal>
   value busy
     color =v
   =imaginal>
     color free
    value t
   )

|#


  
  ;; p1 RHS  static
  
   (p c83
     =imaginal>
     test-num 83
     value nil
     =s nil
     =goal>
     slot1 =s
     !safe-bind! =v 1
     ==>
     =goal>
     =imaginal>
     color =v
     value t)
  
  (p d83
     =imaginal>
     test-num 83
     =s =y
     value t
     =goal>
     slot2 =s
       !safe-bind! =v 1
     ?imaginal>
     state free
   ==>
     =goal>
     !output! =y
     =imaginal>
     =s free
     +imaginal>
     =s =v
     value busy)
    

#|
(p c83+d83
   =imaginal>
   test-num 83
   value nil
    color    nil
   
   =goal>
   slot1 color 
   slot2 color
   
   !safe-bind! =v 1
     ?imaginal>
   state free
   ==>
   =goal>
   
   !output! =v
    +imaginal>
   value busy
     color =v
   =imaginal>
     color free
    value t
   )

|#


    ;; p2 lhs static
  
    (p c84
     =imaginal>
     test-num 84
     value nil
     =s nil
     =goal>
     slot1 =s
     !safe-bind! =v 1
     ==>
     =goal>
     =imaginal>
     =s =v
     value t)
  
  (p d84
     =imaginal>
     test-num 84
     color =y
     value t
     =goal>
     slot2 =s
       !safe-bind! =v 1
     ?imaginal>
     state free
   ==>
     =goal>
     !output! =y
     =imaginal>
     =s free
     +imaginal>
     =s =v
     value busy)
    


#|
(p c84+d84
   =imaginal>
   test-num 84
   value nil
     color   nil
   
   =goal>
   slot1  color
   slot2 color
   
   !safe-bind! =v 1
     ?imaginal>
   state free
   ==>
   =goal>
   
   !output! =v
    +imaginal>
   value busy
   color   =v
   =imaginal>
    color  free
    value t
   )

|#
  ;; p2 rhs = static
  
  (p c85
     =imaginal>
     test-num 85
     value nil
     =s nil
     =goal>
     slot1 =s
     !safe-bind! =v 1
     ==>
     =goal>
     =imaginal>
     =s =v
     value t)
  
  (p d85
     =imaginal>
     test-num 85
     =s =y
     value t
     =goal>
     slot2 =s
        !safe-bind! =v 1
     ?imaginal>
     state free
  ==>
     =goal>
     !output! =y
     =imaginal>
     color free
     +imaginal>
     =s =v
     value busy)
    
  
#|
(p c85+d85
   =imaginal>
   test-num 85
   value nil
    color    nil
   
   =goal>
   slot1  color
   slot2 color
   
   !safe-bind! =v 1
     ?imaginal>
   state free
   ==>
   =goal>
   
   !output! =v
    +imaginal>
   value busy
    color  =v
   =imaginal>
     color free
    value t
   )

|#

  ;; p2 RHS + static
  
    (p c86
     =imaginal>
     test-num 86
     value nil
     =s nil
     =goal>
     slot1 =s
     !safe-bind! =v 1
     ==>
     =goal>
     =imaginal>
     =s =v
     value t)
  
  (p d86
     =imaginal>
     test-num 86
     =s =y
     value t
     =goal>
     slot2 =s
      !safe-bind! =v 1
     ?imaginal>
     state free
    ==>
     =goal>
     !output! =y
     =imaginal>
     =s free
     +imaginal>
     color =v
     value busy)
   
#|
(p c86+d86
   =imaginal>
   test-num 86
   value nil
    =s    nil
   
   =goal>
   slot1 =s 
   slot2 =s
   
   !safe-bind! =v 1
     ?imaginal>
   state free
   ==>
   =goal>
   
   !output! =v
    +imaginal>
   value busy
     color =v
   =imaginal>
     =s free
    value t
   )

|#
 
  ;; 2 static
  
  ;; p1 LHS and RHS static
  
    (p c87
     =imaginal>
     test-num 87
     value nil
     color nil
     =goal>
     slot1 =s
     !safe-bind! =v 1
     ==>
     =goal>
     =imaginal>
     color =v
     value t)
  
  (p d87
     =imaginal>
     test-num 87
     =s =y
     value t
     =goal>
     slot2 =s
         !safe-bind! =v 1
     ?imaginal>
     state free
 ==>
     =goal>
     !output! =y
     =imaginal>
     =s free
     +imaginal>
     =s =v
     value busy)
 

#|
(p c87+d87
   =imaginal>
   test-num 87
   value nil
    color    nil
   
   =goal>
   slot1 =s 
   slot2 color
   
   !safe-bind! =v 1
     ?imaginal>
   state free
   ==>
   =goal>
   
   !output! =v
    +imaginal>
   value busy
     color =v
   =imaginal>
     color free
    value t
   )

|#   
  
    ;; p1 LHS and p2 lhs static
  
   (p c88
     =imaginal>
     test-num 88
     value nil
     color nil
     =goal>
     slot1 =s
     !safe-bind! =v 1
     ==>
     =goal>
     =imaginal>
     =s =v
     value t)
  
  (p d88
     =imaginal>
     test-num 88
     color =y
     value t
     =goal>
     slot2 =s
        !safe-bind! =v 1
     ?imaginal>
     state free
  ==>
     =goal>
     !output! =y
     =imaginal>
     =s free
     +imaginal>
     =s =v
     value busy)
  
#|
(p c88+d88
   =imaginal>
   test-num 88
   value nil
    color    nil
   
   =goal>
   slot1 color 
   slot2 color
   
   !safe-bind! =v 1
     ?imaginal>
   state free
   ==>
   =goal>
   
   !output! =v
    +imaginal>
   value busy
    color  =v
   =imaginal>
     color free
    value t
   )

|#
  
  ;; p1 lhs and p2 rhs = static
  
  (p c89
     =imaginal>
     test-num 89
     value nil
     color nil
     =goal>
     slot1 =s
     !safe-bind! =v 1
     ==>
     =goal>
     =imaginal>
     =s =v
     value t)
  
  (p d89
     =imaginal>
     test-num 89
     =s =y
     value t
     =goal>
     slot2 =s
       !safe-bind! =v 1
     ?imaginal>
     state free
   ==>
     =goal>
     !output! =y
     =imaginal>
     color free
     +imaginal>
     =s =v
     value busy)
    
  #|
(p c89+d89
   =imaginal>
   test-num 89
   value nil
    color    nil
   
   =goal>
   slot1  color
   slot2 color
   
   !safe-bind! =v 1
     ?imaginal>
   state free
   ==>
   =goal>
   
   !output! =v
    +imaginal>
   value busy
     color =v
   =imaginal>
     color free
    value t
   )

|#
  ;; p1 lhs and p2 rhs + static
  
  (p c90
     =imaginal>
     test-num 90
     value nil
     color nil
     =goal>
     slot1 =s
     !safe-bind! =v 1
     ==>
     =goal>
     =imaginal>
     =s =v
     value t)
  
  (p d90
     =imaginal>
     test-num 90
     =s =y
     value t
     =goal>
     slot2 =s
       !safe-bind! =v 1
     ?imaginal>
     state free
   ==>
     =goal>
     !output! =y
     =imaginal>
     =s free
     +imaginal>
     color =v
     value busy)
  
  #|
(p c90+d90
   =imaginal>
   test-num 90
   value nil
    color    nil
   
   =goal>
   slot1  color
   slot2 color
   
   !safe-bind! =v 1
     ?imaginal>
   state free
   ==>
   =goal>
   
   !output! =v
    +imaginal>
   value busy
    color  =v
   =imaginal>
    color  free
    value t
   )

|#
   ;; P1 rhs = p2 lhs static
  
   (p c91
     =imaginal>
     test-num 91
     value nil
     =s nil
     =goal>
     slot1 =s
     !safe-bind! =v 1
     ==>
     =goal>
     =imaginal>
     color =v
     value t)
  
  (p d91
     =imaginal>
     test-num 91
     color =y
     value t
     =goal>
     slot2 =s
        !safe-bind! =v 1
     ?imaginal>
     state free
  ==>
     =goal>
     !output! =y
     =imaginal>
     =s free
     +imaginal>
     =s =v
     value busy)
    
  #|
(p c91+d91
   =imaginal>
   test-num 91
   value nil
     color   nil
   
   =goal>
   slot1  color
   slot2 color
   
   !safe-bind! =v 1
     ?imaginal>
   state free
   ==>
   =goal>
   
   !output! =v
    +imaginal>
   value busy
    color  =v
   =imaginal>
     color free
    value t
   )

|#
  ;; P1 rhs and p2 rhs = static
  
   (p c92
     =imaginal>
     test-num 92
     value nil
     =s nil
     =goal>
     slot1 =s
     !safe-bind! =v 1
     ==>
     =goal>
     =imaginal>
     color =v
     value t)
  
  (p d92
     =imaginal>
     test-num 92
     =s =y
     value t
     =goal>
     slot2 =s
       !safe-bind! =v 1
     ?imaginal>
     state free
   ==>
     =goal>
     !output! =y
     =imaginal>
     color free
     +imaginal>
     =s =v
     value busy)
   #|
(p c92+d92
   =imaginal>
   test-num 92
   value nil
      color  nil
   
   =goal>
   slot1 color  
   slot2 color
   
   !safe-bind! =v 1
     ?imaginal>
   state free
   ==>
   =goal>
   
   !output! =v
    +imaginal>
   value busy
    color  =v
   =imaginal>
    color  free
    value t
   )

|# 
  ;; p1 rhs and p2 rhs + static
  
   (p c93
     =imaginal>
     test-num 93
     value nil
     =s nil
     =goal>
     slot1 =s
     !safe-bind! =v 1
     ==>
     =goal>
     =imaginal>
     color =v
     value t)
  
  (p d93
     =imaginal>
     test-num 93
     =s =y
     value t
     =goal>
     slot2 =s
        !safe-bind! =v 1
     ?imaginal>
     state free
  ==>
     =goal>
     !output! =y
     =imaginal>
     =s free
     +imaginal>
     color =v
     value busy)
    
   #|
(p c93+d93
   =imaginal>
   test-num 93
   value nil
    color    nil
   
   =goal>
   slot1  color
   slot2 color
   
   !safe-bind! =v 1
        ?imaginal>
   state free
   ==>
   =goal>
   
   !output! =v
    +imaginal>
   value busy
    color  =v
   =imaginal>
     color free
    value t
   )

|#
 ;; p2 lhs and rhs = static
  
   (p c94
     =imaginal>
     test-num 94
     value nil
     =s nil
     =goal>
     slot1 =s
     !safe-bind! =v 1
     ==>
     =goal>
     =imaginal>
     =s =v
     value t)
  
  (p d94
     =imaginal>
     test-num 94
     color =y
     value t
     =goal>
     slot2 =s
        !safe-bind! =v 1
       ?imaginal>
     state free
==>
     =goal>
     !output! =y
     =imaginal>
     color free
     +imaginal>
     =s =v
     value busy)
   
  #|
(p c94+d94
   =imaginal>
   test-num 94
   value nil
    color    nil
   
   =goal>
   slot1  color
   slot2 =s
   
   !safe-bind! =v 1
        ?imaginal>
   state free
   ==>
   =goal>
   
   !output! =v
    +imaginal>
   value busy
    =s  =v
   =imaginal>
     color free
    value t
   )

|#
  ;; p2 lhs rhs + static
  
  (p c95
     =imaginal>
     test-num 95
     value nil
     =s nil
     =goal>
     slot1 =s
     !safe-bind! =v 1
     ==>
     =goal>
     =imaginal>
     =s =v
     value t)
  
  (p d95
     =imaginal>
     test-num 95
     color =y
     value t
     =goal>
     slot2 =s
       !safe-bind! =v 1
        ?imaginal>
     state free
==>
     =goal>
     !output! =y
     =imaginal>
     =s free
     +imaginal>
     color =v
     value busy)
    
  #|
(p c95+d95
   =imaginal>
   test-num 95
   value nil
   color     nil
   
   =goal>
   slot1 color  
   slot2 color
   
   !safe-bind! =v 1
        ?imaginal>
   state free
   ==>
   =goal>
   
   !output! =v
    +imaginal>
   value busy
    color  =v
   =imaginal>
     color free
    value t
   )

|#
    ;; p2  rhs = + static
  
   (p c96
     =imaginal>
     test-num 96
     value nil
     =s nil
     =goal>
     slot1 =s
     !safe-bind! =v 1
     ==>
     =goal>
     =imaginal>
     =s =v
     value t)
  
  (p d96
     =imaginal>
     test-num 96
     =s =y
     value t
     =goal>
     slot2 =s
        !safe-bind! =v 1
       ?imaginal>
     state free
==>
     =goal>
     !output! =y
     =imaginal>
     color free
     +imaginal>
     color =v
     value busy)
    
  #|
(p c96+d96
   =imaginal>
   test-num 96
   value nil
   color     nil
   
   =goal>
   slot1  color
   slot2 color
   
   !safe-bind! =v 1
        ?imaginal>
   state free
   ==>
   =goal>
   
   !output! =v
    +imaginal>
   value busy
    color  =v
   =imaginal>
     color free
    value t
   )

|#
    ;; 3 static
  
  
    ;; p1 LHS and RHS p2 lhs static
  
    (p c97
     =imaginal>
     test-num 97
     value nil
     color nil
     =goal>
     slot1 =s
     !safe-bind! =v 1
     ==>
     =goal>
     =imaginal>
     color =v
     value t)
  
  (p d97
     =imaginal>
     test-num 97
     color =y
     value t
     =goal>
     slot2 =s
       !safe-bind! =v 1
        ?imaginal>
     state free
==>
     =goal>
     !output! =y
     =imaginal>
     =s free
     +imaginal>
     =s =v
     value busy)
    
  #|
(p c97+d97
   =imaginal>
   test-num 83
   value nil
     color   nil
   
   =goal>
   slot1  =s
   slot2 color
   
   !safe-bind! =v 1
        ?imaginal>
   state free
   ==>
   =goal>
   
   !output! =v
    +imaginal>
   value busy
    color  =v
   =imaginal>
     color free
    value t
   )

|#
    ;; p1 LHS RHS p2 rhs = static
  
   (p c98
     =imaginal>
     test-num 98
     value nil
     color nil
     =goal>
     slot1 =s
     !safe-bind! =v 1
     ==>
     =goal>
     =imaginal>
     color =v
     value t)
  
  (p d98
     =imaginal>
     test-num 98
     =s =y
     value t
     =goal>
     slot2 =s
      !safe-bind! =v 1
         ?imaginal>
     state free
==>
     =goal>
     !output! =y
     =imaginal>
     color free
     +imaginal>
     =s =v
     value busy)
 #|
(p c98+d98
   =imaginal>
   test-num 98
   value nil
   color     nil
   
   =goal>
   slot1 =s 
   slot2 color
   
   !safe-bind! =v 1
        ?imaginal>
   state free
   ==>
   =goal>
   
   !output! =v
    +imaginal>
   value busy
    color  =v
   =imaginal>
     color free
    value t
   )

|#   
  ;; p1 lhs rhs and p2 rhs + static
  
   (p c99
     =imaginal>
     test-num 99
     value nil
     color nil
     =goal>
     slot1 =s
     !safe-bind! =v 1
     ==>
     =goal>
     =imaginal>
     color =v
     value t)
  
  (p d99
     =imaginal>
     test-num 99
     =s =y
     value t
     =goal>
     slot2 =s
       !safe-bind! =v 1
        ?imaginal>
     state free
==>
     =goal>
     !output! =y
     =imaginal>
     =s free
     +imaginal>
     color =v
     value busy)
 
#|
(p c99+d99
   =imaginal>
   test-num 99
   value nil
   color     nil
   
   =goal>
   slot1  =s
   slot2 color
   
   !safe-bind! =v 1
        ?imaginal>
   state free
   ==>
   =goal>
   
   !output! =v
    +imaginal>
   value busy
     color =v
   =imaginal>
    color  free
    value t
   )

|#   
  ;; p1 lhs and p2 lhs rhs = static
  
   (p c100
     =imaginal>
     test-num 100
     value nil
     color nil
     =goal>
     slot1 =s
     !safe-bind! =v 1
     ==>
     =goal>
     =imaginal>
     =s =v
     value t)
  
  (p d100
     =imaginal>
     test-num 100
     color =y
     value t
     =goal>
     slot2 =s
      !safe-bind! =v 1
         ?imaginal>
     state free
==>
     =goal>
     !output! =y
     =imaginal>
     color free
     +imaginal>
     =s =v
     value busy)
  
  #|
(p c100+d100
   =imaginal>
   test-num 100
   value nil
   color     nil
   
   =goal>
   slot1  color
   slot2 =s
   
   !safe-bind! =v 1
        ?imaginal>
   state free
   ==>
   =goal>
   
   !output! =v
    +imaginal>
   value busy
    =s  =v
   =imaginal>
    color  free
    value t
   )

|#
  
   ;; P1 lhs p2 lhs rhs + static
  
   (p c101
     =imaginal>
     test-num 101
     value nil
     color nil
     =goal>
     slot1 =s
     !safe-bind! =v 1
     ==>
     =goal>
     =imaginal>
     =s =v
     value t)
  
  (p d101
     =imaginal>
     test-num 101
     color =y
     value t
     =goal>
     slot2 =s
      !safe-bind! =v 1
         ?imaginal>
     state free
==>
     =goal>
     !output! =y
     =imaginal>
     =s free
     +imaginal>
     color =v
     value busy)
  

  #|
(p c101+d101
   =imaginal>
   test-num 101
   value nil
    color    nil
   
   =goal>
   slot1  color
   slot2 color
   
   !safe-bind! =v 1
        ?imaginal>
   state free
   ==>
   =goal>
   
   !output! =v
    +imaginal>
   value busy
    color  =v
   =imaginal>
     color free
    value t
   )

|#
  
  ;; P1 lhs and p2 rhs = + static
  
   (p c102
     =imaginal>
     test-num 102
     value nil
     color nil
     =goal>
     slot1 =s
     !safe-bind! =v 1
     ==>
     =goal>
     =imaginal>
     =s =v
     value t)
  
  (p d102
     =imaginal>
     test-num 102
     =s =y
     value t
     =goal>
     slot2 =s
     !safe-bind! =v 1
          ?imaginal>
     state free
==>
     =goal>
     !output! =y
     =imaginal>
     color free
     +imaginal>
     color =v
     value busy)

    #|
(p c102+d102
   =imaginal>
   test-num 102
   value nil
    color    nil
   
   =goal>
   slot1  color
   slot2 color
   
   !safe-bind! =v 1
       ?imaginal>
   state free
   ==>
   =goal>
   
   !output! =v
    +imaginal>
   value busy
    color  =v
   =imaginal>
    color  free
    value t
   )

|#
  
  ;; p1 rhs and p2 lhs rhs = static
  
   (p c103
     =imaginal>
     test-num 103
     value nil
     =s nil
     =goal>
     slot1 =s
     !safe-bind! =v 1
     ==>
     =goal>
     =imaginal>
     color =v
     value t)
  
  (p d103
     =imaginal>
     test-num 103
     color =y
     value t
     =goal>
     slot2 =s
       !safe-bind! =v 1
        ?imaginal>
     state free
==>
     =goal>
     !output! =y
     =imaginal>
     color free
     +imaginal>
     =s =v
     value busy)
 
  #|
(p c103+d103
   =imaginal>
   test-num 103
   value nil
    color    nil
   
   =goal>
   slot1  color
   slot2 =s2
   
   !safe-bind! =v 1
        ?imaginal>
   state free
   ==>
   =goal>
   
   !output! =v
    +imaginal>
   value busy
    =s2  =v
   =imaginal>
    color  free
    value t
   )

|#
   
    ;; P1 rhs p2 lhs rhs + static
  
   (p c104
     =imaginal>
     test-num 104
     value nil
     =s nil
     =goal>
     slot1 =s
     !safe-bind! =v 1
     ==>
     =goal>
     =imaginal>
     color =v
     value t)
  
  (p d104
     =imaginal>
     test-num 104
     color =y
     value t
     =goal>
     slot2 =s
      !safe-bind! =v 1
        ?imaginal>
     state free
 ==>
     =goal>
     !output! =y
     =imaginal>
     =s free
     +imaginal>
     color =v
     value busy)
   
   #|
(p c104+d104
   =imaginal>
   test-num 104
   value nil
     color   nil
   
   =goal>
   slot1  color
   slot2 color
   
   !safe-bind! =v 1
        ?imaginal>
   state free
   ==>
   =goal>
   
   !output! =v
    +imaginal>
   value busy
    color  =v
   =imaginal>
     color free
    value t
   )

|#
 
  ;; p1 rhs and p2 rhs = + static
  
   (p c105
     =imaginal>
     test-num 105
     value nil
     =s nil
     =goal>
     slot1 =s
     !safe-bind! =v 1
     ==>
     =goal>
     =imaginal>
     color =v
     value t)
  
  (p d105
     =imaginal>
     test-num 105
     =s =y
     value t
     =goal>
     slot2 =s
     !safe-bind! =v 1
          ?imaginal>
     state free
==>
     =goal>
     !output! =y
     =imaginal>
     color free
     +imaginal>
     color =v
     value busy)
 
  #|
(p c105+d105
   =imaginal>
   test-num 105
   value nil
    color    nil
   
   =goal>
   slot1  color
   slot2 color
   
   !safe-bind! =v 1
        ?imaginal>
   state free
   ==>
   =goal>
   
   !output! =v
    +imaginal>
   value busy
    color  =v
   =imaginal>
    color  free
    value t
   )

|#
   
    
    ;; p2 lhs and rhs = + static
  
   (p c106
     =imaginal>
     test-num 106
     value nil
     =s nil
     =goal>
     slot1 =s
     !safe-bind! =v 1
     ==>
     =goal>
     =imaginal>
     =s =v
     value t)
  
  (p d106
     =imaginal>
     test-num 106
     color =y
     value t
     =goal>
     slot2 =s
      !safe-bind! =v 1
         ?imaginal>
     state free
==>
     =goal>
     !output! =y
     =imaginal>
     color free
     +imaginal>
     color =v
     value busy)
    
     #|
(p c106+d106
   =imaginal>
   test-num 106
   value nil
    color    nil
   
   =goal>
   slot1  color
   slot2 =s
   
   !safe-bind! =v 1
        ?imaginal>
   state free
   ==>
   =goal>
   
   !output! =v
    +imaginal>
   value busy
    color  =v
   =imaginal>
    color  free
    value t
   )

|#
 
  
  
  
  ;; 4 static
  
   
  
  ;; only p1 lhs dynamic
  
   (p c107
     =imaginal>
     test-num 107
     value nil
     =s nil
     =goal>
     slot1 =s
     !safe-bind! =v 1
     ==>
     =goal>
     =imaginal>
     color =v
     value t)
  
  (p d107
     =imaginal>
     test-num 107
     color =y
     value t
     =goal>
     slot2 =s
      !safe-bind! =v 1
        ?imaginal>
     state free
 ==>
     =goal>
     !output! =y
     =imaginal>
     color free
     +imaginal>
     color =v
     value busy)
    
    #|
(p c107+d107
   =imaginal>
   test-num 107
   value nil
     color   nil
   
   =goal>
   slot1  color
   slot2 =s2
   
   !safe-bind! =v 1
        ?imaginal>
   state free
   ==>
   =goal>
   
   !output! =v
    +imaginal>
   value busy
     color =v
   =imaginal>
    color  free
    value t
   )

|#
    
  ;; only p1 rhs  dynamic
  
   (p c108
     =imaginal>
     test-num 108
     value nil
     color nil
     =goal>
     slot1 =s
     !safe-bind! =v 1
     ==>
     =goal>
     =imaginal>
     =s =v
     value t)
  
  (p d108
     =imaginal>
     test-num 108
     color =y
     value t
     =goal>
     slot2 =s
      !safe-bind! =v 1
         ?imaginal>
     state free
==>
     =goal>
     !output! =y
     =imaginal>
     color free
     +imaginal>
     color =v
     value busy)

    #|
(p c108+d108
   =imaginal>
   test-num 108
   value nil
   color     nil
   
   =goal>
   slot1  color
   slot2 =s
   
   !safe-bind! =v 1
        ?imaginal>
   state free
   ==>
   =goal>
   
   !output! =v
    +imaginal>
   value busy
     color =v
   =imaginal>
     color free
    value t
   )

|#
  
    ;; only p2 lhs dynamic
  
   (p c109
     =imaginal>
     test-num 109
     value nil
     color nil
     =goal>
     slot1 =s
     !safe-bind! =v 1
     ==>
     =goal>
     =imaginal>
     color =v
     value t)
  
  (p d109
     =imaginal>
     test-num 109
     =s =y
     value t
     =goal>
     slot2 =s
     !safe-bind! =v 1
          ?imaginal>
     state free
==>
     =goal>
     !output! =y
     =imaginal>
     color free
     +imaginal>
     color =v
     value busy)
    
    #|
(p c109+d109
   =imaginal>
   test-num 109
   value nil
    color    nil
   
   =goal>
   slot1  =s
   slot2 color
   
   !safe-bind! =v 1
        ?imaginal>
   state free
   ==>
   =goal>
   
   !output! =v
    +imaginal>
   value busy
    color  =v
   =imaginal>
    color  free
    value t
   )

|#

  ;; only p2 rhs = dynamic
  
   (p c110
     =imaginal>
     test-num 110
     value nil
     color nil
     =goal>
     slot1 =s
     !safe-bind! =v 1
     ==>
     =goal>
     =imaginal>
     color =v
     value t)
  
  (p d110
     =imaginal>
     test-num 110
     color =y
     value t
     =goal>
     slot2 =s
     !safe-bind! =v 1
          ?imaginal>
     state free
==>
     =goal>
     !output! =y
     =imaginal>
     =s free
     +imaginal>
     color =v
     value busy)
   

  #|
(p c110+d110
   =imaginal>
   test-num 110
   value nil
    color    nil
   
   =goal>
   slot1 =s 
   slot2 color
   
   !safe-bind! =v 1
       ?imaginal>
   state free
   ==>
   =goal>
   
   !output! =v
    +imaginal>
   value busy
    color  =v
   =imaginal>
     color free
    value t
   )

|#

   
    ;; only p2 rhs + dynamic
  
    (p c111
     =imaginal>
     test-num 111
     value nil
     color nil
     =goal>
     slot1 =s
     !safe-bind! =v 1
     ==>
     =goal>
     =imaginal>
     color =v
     value t)
  
  (p d111
     =imaginal>
     test-num 111
     color =y
     value t
     =goal>
     slot2 =s
     !safe-bind! =v 1
         ?imaginal>
     state free
 ==>
     =goal>
     !output! =y
     =imaginal>
     color free
     +imaginal>
     =s =v
     value busy)
    
  
    #|
(p c111+d111
   =imaginal>
   test-num 111
   value nil
   color     nil
   
   =goal>
   slot1 =s  
   slot2 =s2
   
   !safe-bind! =v 1
       ?imaginal>
   state free
   ==>
   =goal>
   
   !output! =v
    +imaginal>
   value busy
    =s2  =v
   =imaginal>
    color  free
    value t
   )

|#

  ;;; p1 lhs = with p2 = , =
  
  ;; all dynamic
  
  (p c112
     =imaginal>
     test-num 112
     value nil
     =s nil
     =goal>
     slot1 =s
     test-num 112
     ==>
     =goal>
     test-num t
     )
  
  (p d112
     =imaginal>
     test-num 112
     =s nil
     =goal>
     test-num t
     slot2 =s
     !safe-bind! =v 1
     ==>
     =goal>
     =imaginal>
     =s =v)
  
    #|


(p c112+d112
   =imaginal>
   test-num 112
   value nil
    =s    nil
    =s1   nil
   =goal>
   slot1  =s
   slot2 =s1
   test-num 112
   
   !safe-bind! =v 1
   ==>
   =goal>
   test-num t
   =imaginal>
    =s1  =v
   )

|#

  ;; one static
  
  ;; p1 lhs static
  
  (p c113
     =imaginal>
     test-num 113
     value nil
     color nil
     =goal>
     slot1 =s
     test-num 113
     ==>
     =goal>
     test-num t
     )
  
  (p d113
     =imaginal>
     test-num 113
     =s nil
     =goal>
     test-num t
     slot2 =s
     !safe-bind! =v 1
     ==>
     =goal>
     =imaginal>
     =s =v)

    #|

  same issue as a&b 113


(p c113+d113
   =imaginal>
   test-num 113
   value nil
    =s2    nil
    color  nil

   =goal>
   slot1  =s
   slot2 =s2
   test-num 113
   
   !safe-bind! =v 1
   ==>
   =goal>
   test-num t
   =imaginal>
    =s2  =v
   )

|#



  ;; p2 lhs static
  
    (p c114
     =imaginal>
     test-num 114
     value nil
     =s nil
     =goal>
     slot1 =s
     test-num 114
     ==>
     =goal>
     test-num t
     )
  
  (p d114
     =imaginal>
     test-num 114
     color nil
     =goal>
     test-num t
     slot2 =s
     !safe-bind! =v 1
     ==>
     =goal>
     =imaginal>
     =s =v)
  
      #|

  same issue as a&b 114
(p c114+d114
   =imaginal>
   test-num 114
   value nil
     color   nil
    =s nil
   =goal>
   slot1  =s
   slot2 =s1
   test-num 114
   
   !safe-bind! =v 1
   ==>
   =goal>
   test-num t
   =imaginal>
   =s1   =v
   )

|#

;; p2 rhs static
  
    (p c115
     =imaginal>
     test-num 115
     value nil
     =s nil
     =goal>
     slot1 =s
     test-num 115
     ==>
     =goal>
     test-num t
     )
  
  (p d115
     =imaginal>
     test-num 115
     =s nil
     =goal>
     test-num t
     slot2 =s
     !safe-bind! =v 1
     ==>
     =goal>
     =imaginal>
     color =v)
  

    #|
(p c115+d115
   =imaginal>
   test-num 115
   value nil
   =s    nil
   =s2 nil
   =goal>
   slot1  =s
   slot2 =s2
   test-num 115
   
   !safe-bind! =v 1
   ==>
   =goal>
   test-num t
   =imaginal>
    color  =v
   )

|#
  ;; two static
  
  ;; p1 lhs p2 lhs
  
    (p c116
     =imaginal>
     test-num 116
     value nil
     color nil
     =goal>
     slot1 =s
     test-num 116
     ==>
     =goal>
     test-num t
     )
  
  (p d116
     =imaginal>
     test-num 116
     color nil
     =goal>
     test-num t
     slot2 =s
     !safe-bind! =v 1
     ==>
     =goal>
     =imaginal>
     =s =v)
  
    #|
(p c116+d116
   =imaginal>
   test-num 116
   value nil
    color    nil
   
   =goal>
   slot1  =s
   slot2 =s1
   test-num 116
   
   !safe-bind! =v 1
   ==>
   =goal>
   test-num t
   =imaginal>
    =s1  =v
   )

|#


  ;; p1 lhs p2 rhs
  
    (p c117
     =imaginal>
     test-num 117
     value nil
     color nil
     =goal>
     slot1 =s
     test-num 117
     ==>
     =goal>
     test-num t
     )
  
  (p d117
     =imaginal>
     test-num 117
     =s nil
     =goal>
     test-num t
     slot2 =s
     !safe-bind! =v 1
     ==>
     =goal>
     =imaginal>
     color =v)
  
    #|
(p c117+d117
   =imaginal>
   test-num 117
   value nil
    color    nil
   =s2  nil
   =goal>
   slot1 =s 
   slot2 =s2
   test-num 117
   
   !safe-bind! =v 1
   ==>
   =goal>
   test-num t
   =imaginal>
    color  =v
   )

|#


  ;; p2 lhs rhs
  
    (p c118
     =imaginal>
     test-num 118
     value nil
     =s nil
     =goal>
     slot1 =s
     test-num 118
     ==>
     =goal>
     test-num t
     )
  
  (p d118
     =imaginal>
     test-num 118
     color nil
     =goal>
     test-num t
     slot2 =s
     !safe-bind! =v 1
     ==>
     =goal>
     =imaginal>
     color =v)

      #|
(p c118+d118
   =imaginal>
   test-num 118
   value nil
    color    nil
    =s  nil
   =goal>
   slot1  =s
   slot2 =s1
   test-num 118
   
   !safe-bind! =v 1
   ==>
   =goal>
   test-num t
   =imaginal>
    color  =v
   )

|#
  
  
  ;;; p1 rhs +  with p2 =, =
  
  
  (p c119
     =goal>
     slot1 =s
     test-num 119
         ?imaginal>
     state free
 ==>
     =goal>
     test-num t
     +imaginal>
     =s 119
     )
  
  (p d119
     =imaginal>
     =s 119
     =goal>
     test-num t
     slot2 =s
     ==>
     =goal>
     =imaginal>
     =s free)
  
    #|
(p c119+d119
   
   =goal>
   slot1  =s
   slot2 =s
   test-num 119
       ?imaginal>
     state free 
   ==>
   =goal>
   test-num t
  
   +imaginal>
    =s  free
   )

|#

  
  ;; one static
  
  ;; p1 rhs
  
  
    (p c120
     =goal>
     slot1 =s
     test-num 120
         ?imaginal>
     state free
 ==>
     =goal>
     test-num t
     +imaginal>
     color 120
     )
  
  (p d120
     =imaginal>
     =s 120
     =goal>
     test-num t
     slot2 =s
     ==>
     =goal>
     =imaginal>
     =s free)
  
    #|
(p c120+d120
   
   =goal>
   slot1  =s
   slot2 color
   test-num 120
        ?imaginal>
     state free
   ==>
   =goal>
   test-num t
  
   +imaginal>
    color  free
   )

|#


  ;; p2 lhs
  
    (p c121
     =goal>
     slot1 =s
     test-num 121
        ?imaginal>
     state free
  ==>
     =goal>
     test-num t
     +imaginal>
     =s 121
     )
  
  (p d121
     =imaginal>
     color 121
     =goal>
     test-num t
     slot2 =s
     ==>
     =goal>
     =imaginal>
     =s free)
  
    #|


(p c121+d121
   
   =goal>
   slot1  color
   slot2 color
   test-num 121
        ?imaginal>
     state free
   ==>
   =goal>
   test-num t
  
   +imaginal>
   color free
   )

|#


  ;; p2 rhs
  
    (p c122
     =goal>
     slot1 =s
     test-num 122
        ?imaginal>
     state free
  ==>
     =goal>
     test-num t
     +imaginal>
     =s 122
     )
  
  (p d122
     =imaginal>
     =s 122
     =goal>
     test-num t
     slot2 =s
     ==>
     =goal>
     =imaginal>
     color free)
  

    #|
(p c122+d122
   
   =goal>
   slot1 color
   slot2 color
   test-num 122
        ?imaginal>
     state free
   ==>
   =goal>
   test-num t
  
   +imaginal>
   color   free
   )

|#


  ;; two static
  
  ;; p1 rhs p2 lhs
  
    (p c123
     =goal>
     slot1 =s
     test-num 123
        ?imaginal>
     state free
  ==>
     =goal>
     test-num t
     +imaginal>
     color 123
     )
  
  (p d123
     =imaginal>
     color 123
     =goal>
     test-num t
     slot2 =s
     ==>
     =goal>
     =imaginal>
     =s free)
  
    #|
(p c123+d123
   
   =goal>
   slot1 =s 
   slot2 color 
   test-num 123
       ?imaginal>
     state free 
   ==>
   
   =goal>
   test-num t
  
   +imaginal>
     color free
   )

|#



  ;; p1 rhs p2 rhs
  
    (p c124
     =goal>
     slot1 =s
     test-num 124
       ?imaginal>
     state free
   ==>
     =goal>
     test-num t
     +imaginal>
     color 124
     )
  
  (p d124
     =imaginal>
     =s 124
     =goal>
     test-num t
     slot2 =s
     ==>
     =goal>
     =imaginal>
     color free)
  
    #|
(p c124+d124
   
   =goal>
   slot1 =s 
   slot2 color 
   test-num 124
        ?imaginal>
     state free
   ==>
   
   =goal>
   test-num t
  
   +imaginal>
     color free
   )

|#


  ;; p2 lhs rhs
  
    (p c125
     =goal>
     slot1 =s
     test-num 125
         ?imaginal>
     state free
 ==>
     =goal>
     test-num t
     +imaginal>
     =s 125
     )
  
  (p d125
     =imaginal>
     color 125
     =goal>
     test-num t
     slot2 =s
     ==>
     =goal>
     =imaginal>
     color free)
  
  
      #|
(p c125+d125
   
   =goal>
   slot1 color 
   slot2 =s 
   test-num 125
        ?imaginal>
     state free
   ==>
   
   =goal>
   test-num t
  
   +imaginal>
     color free
   )

|#



  
  ;;; Here's a test with multiple dynamics
  
  ;(imaginal-focus-fct (car (define-chunks (value 1))))
  ;(set-buffer-chunk 'goal (car (define-chunks (slot1 color slot2 color slot3 value))))
  
  ;; all dynamic
  
  (p c126
     =imaginal>
     =s3 1
     =s1 nil
     test-num 126
     =goal>
     slot1 =s1
     slot3 =s3
     ==>
     =imaginal>
     =s1 blue
     =goal>)

  (p d126
     =imaginal>
     test-num 126
     =s2 blue
     =goal>
     slot2 =s2
     slot3 =s3
     ==>
     =imaginal>
     =s2 nil
     =s3 2)



#|

(p c126+d126
   =imaginal>
   test-num 126
    =s3  1
    =s1  nil
   =goal>
   slot1 =s1
   slot2 =s1
   slot3 =s3
   ==>
   =imaginal>
    =s1  nil
    =s3  2
  =goal>
  )
|#

  ;; make p2 lhs static
  
  (p c127
     =imaginal>
     test-num 127
     =s3 1
     =s1 nil
     =goal>
     slot1 =s1
     slot3 =s3
     ==>
     =imaginal>
     =s1 blue
     =goal>)
  
  (p d127
     =imaginal>
     test-num 127
     color blue
     =goal>
     slot2 =s2
     slot3 =s3
     ==>
     =imaginal>
     =s2 nil
     =s3 2)

 #|

(p c127+d127
   =imaginal>
   test-num 127
    =s3  1
   color   nil
   =goal>
   slot1 color
   slot2 color
   slot3 =s3
   ==>
   =goal>
   =imaginal>
    color  nil
    =s3  2
  )
|#
 
  
  
  ;;; Test case where dynamics don't match between productions
  
    ;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;
  ;; Tests with 4 possible dynamic location
  ;; LHS,RHS = & LHS,RHS =
  
  ;; slot4 has value slot4
  
  ;; all 4 dynamic
  
  (p c128
     =imaginal>
     test-num 128
     value nil
     =s1 nil
     
     =goal>
     slot1 =s1
     
     !safe-bind! =v 1
     ==>
     =goal>
     =imaginal>
     =s1 =v
     value t)
  
  (p d128
     =imaginal>
     test-num 128
     =s4 nil
     value t
     =goal>
     slot4 =s4
     ==>
     =goal>
     =imaginal>
     =s4 free
     value busy)
  

#|
(p c128+d128
   =imaginal>
   test-num 128
   value nil
   =s1     nil
   =s4     nil
   =goal>
    slot1 =s1
   slot4 =s4
   !safe-bind! =v 1
   ==>
   =goal>
   =imaginal>
   value busy
   =s1 =v
   =s4 free)
|#

  ;; p1 LHS static
  
    (p c129
     =imaginal>
     test-num 129
     value nil
     color nil
     
     =goal>
     slot1 =s1
     
     !safe-bind! =v 1
     ==>
     =goal>
     =imaginal>
     =s1 =v
     value t)
  
  (p d129
     =imaginal>
     test-num 129
     =s4 nil
     value t
     =goal>
     slot4 =s4
     ==>
     =goal>
     =imaginal>
     =s4 free
     value busy)
  
#|
(p c129+d129
   =imaginal>
   test-num 129
   value nil
   color     nil
   =s4     nil
   =goal>
    slot1 =s1
   slot4 =s4
   !safe-bind! =v 1
   ==>
   =goal>
   =imaginal>
   value busy
   =s1 =v
   =s4 free)
|#

  
  ;; p1 RHS static
  
    (p c130
     =imaginal>
     test-num 130
     value nil
     =s1 nil
     
     =goal>
     slot1 =s1
     
     !safe-bind! =v 1
     ==>
     =goal>
     =imaginal>
     color =v
     value t)
  
  (p d130
     =imaginal>
     test-num 130
     =s4 nil
     value t
     =goal>
     slot4 =s4
     ==>
     =goal>
     =imaginal>
     =s4 free
     value busy)
  

#|
(p c130+d130
   =imaginal>
   test-num 130
   value nil
   =s1     nil
   =s4     nil
   =goal>
    slot1 =s1
   slot4 =s4
   !safe-bind! =v 1
   ==>
   =goal>
   =imaginal>
   value busy
   color =v
   =s4 free)
|#

  
  ;; p2 LHS static
  
    (p c131
     =imaginal>
     test-num 131
     value nil
     =s1 nil
     
     =goal>
     slot1 =s1
     
     !safe-bind! =v 1
     ==>
     =goal>
     =imaginal>
     =s1 =v
     value t)
  
  (p d131
     =imaginal>
     test-num 131
     slot4 nil
     value t
     =goal>
     slot4 =s4
     ==>
     =goal>
     =imaginal>
     =s4 free
     value busy)
    
#|
(p c131+d131
   =imaginal>
   test-num 131
   value nil
   =s1     nil
   slot4     nil
   =goal>
    slot1 =s1
   slot4 =s4
   !safe-bind! =v 1
   ==>
   =goal>
   =imaginal>
   value busy
   =s1 =v
   =s4 free)
|#



  ;; p2 RHS static
  
   (p c132
     =imaginal>
     test-num 132
     value nil
     =s1 nil
     
     =goal>
     slot1 =s1
     
     !safe-bind! =v 1
     ==>
     =goal>
     =imaginal>
     =s1 =v
     value t)
  
  (p d132
     =imaginal>
     test-num 132
     =s4 nil
     value t
     =goal>
     slot4 =s4
     ==>
     =goal>
     =imaginal>
     slot4 free
     value busy)
    
#|
(p c132+d132
   =imaginal>
   test-num 132
   value nil
   =s1     nil
   =s4     nil
   =goal>
    slot1 =s1
   slot4 =s4
   !safe-bind! =v 1
   ==>
   =goal>
   =imaginal>
   value busy
   =s1 =v
   slot4 free)
|#
  


  ;; p1 LHS and RHS static
  
   (p c133
     =imaginal>
     test-num 133
     value nil
     color nil
     
     =goal>
     slot1 =s1
     !safe-bind! =v 1
     ==>
     =goal>
     =imaginal>
     color =v
     value t)
  
  (p d133
     =imaginal>
     test-num 133
     =s4 nil
     value t
     =goal>
     slot4 =s4
     ==>
     =goal>
     =imaginal>
     =s4 free
     value busy)
   
#|
(p c133+d133
   =imaginal>
   test-num 133
   value nil
   color     nil
   =s4     nil
   =goal>
    slot1 =s1
   slot4 =s4
   !safe-bind! =v 1
   ==>
   =goal>
   =imaginal>
   value busy
   color =v
   =s4 free)
|#

 
  ;; p1 lhs and p2 lhs static
  
    (p c134
     =imaginal>
     test-num 134
     value nil
     color nil
     
     =goal>
     slot1 =s1
     
     !safe-bind! =v 1
     ==>
     =goal>
     =imaginal>
     =s1 =v
     value t)
  
  (p d134
     =imaginal>
     test-num 134
     slot4 nil
     value t
     =goal>
     slot4 =s4
     ==>
     =goal>
     =imaginal>
     =s4 free
     value busy)
    
  
#|
(p c134+d134
   =imaginal>
   test-num 134
   value nil
   color     nil
   slot4     nil
   =goal>
    slot1 =s1
   slot4 =s4
   !safe-bind! =v 1
   ==>
   =goal>
   =imaginal>
   value busy
   =s1 =v
   =s4 free)
|#

  
  ;; p1 lhs and p2 rhs static
  
     (p c135
     =imaginal>
     test-num 135
     value nil
     color nil
     
     =goal>
     slot1 =s1
     
     !safe-bind! =v 1
     ==>
     =goal>
     =imaginal>
     =s1 =v
     value t)
  
  (p d135
     =imaginal>
     test-num 135
     =s4 nil
     value t
     =goal>
     slot4 =s4
     ==>
     =goal>
     =imaginal>
     slot4 free
     value busy)
   
 #|
(p c135+d135
   =imaginal>
   test-num 135
   value nil
   color     nil
   =s4     nil
   =goal>
    slot1 =s1
   slot4 =s4
   !safe-bind! =v 1
   ==>
   =goal>
   =imaginal>
   value busy
   =s1 =v
   slot4 free)
|#
 
  
  ;; P1 rhs and p2 lhs static
  
  (p c136
     =imaginal>
     test-num 136
     value nil
     =s1 nil
     
     =goal>
     slot1 =s1
     
     !safe-bind! =v 1
     ==>
     =goal>
     =imaginal>
     color =v
     value t)
  
  (p d136
     =imaginal>
     test-num 136
     slot4 nil
     value t
     =goal>
     slot4 =s4
     ==>
     =goal>
     =imaginal>
     =s4 free
     value busy)
#|
(p c136+d136
   =imaginal>
   test-num 136
   value nil
   =s1     nil
   slot4     nil
   =goal>
    slot1 =s1
   slot4 =s4
   !safe-bind! =v 1
   ==>
   =goal>
   =imaginal>
   value busy
   color =v
   =s4 free)
|#
    
  ;; p1 rhs and p2 rhs static
  
   (p c137
     =imaginal>
     test-num 137
     value nil
     =s1 nil
     
     =goal>
     slot1 =s1
     
     !safe-bind! =v 1
     ==>
     =goal>
     =imaginal>
     color =v
     value t)
  
  (p d137
     =imaginal>
     test-num 137
     =s4 nil
     value t
     =goal>
     slot4 =s4
     ==>
     =goal>
     =imaginal>
     slot4 free
     value busy)

#|
(p c137+d137
   =imaginal>
   test-num 137
   value nil
   =s1     nil
   =s4     nil
   =goal>
    slot1 =s1
   slot4 =s4
   !safe-bind! =v 1
   ==>
   =goal>
   =imaginal>
   value busy
   color =v
   slot4 free)
|#
    
    ;; p2 lhs and rhs static
  
   (p c138
     =imaginal>
     test-num 138
     value nil
     =s1 nil
     
     =goal>
     slot1 =s1
     
     !safe-bind! =v 1
     ==>
     =goal>
     =imaginal>
     =s1 =v
     value t)
  
  (p d138
     =imaginal>
     test-num 138
     slot4 nil
     value t
     =goal>
     slot4 =s4
     ==>
     =goal>
     =imaginal>
     slot4 free
     value busy)

   #|
(p c138+d138
   =imaginal>
   test-num 138
   value nil
   =s1     nil
   slot4     nil
   =goal>
    slot1 =s1
   slot4 =s4
   !safe-bind! =v 1
   ==>
   =goal>
   =imaginal>
   value busy
   =s1 =v
   slot4 free)
|#
 
  
  ;; only p1 lhs dynamic
  
   (p c139
     =imaginal>
     test-num 139
     value nil
     =s1 nil
     
     =goal>
     slot1 =s1
     
     !safe-bind! =v 1
     ==>
     =goal>
     =imaginal>
     color =v
     value t)
  
  (p d139
     =imaginal>
     test-num 139
     slot4 nil
     value t
     =goal>
     slot4 =s4
     ==>
     =goal>
     =imaginal>
     slot4 free
     value busy)
    
  #|
(p c139+d139
   =imaginal>
   test-num 139
   value nil
   =s1     nil
   slot4     nil
   =goal>
    slot1 =s1
   slot4 =s4
   !safe-bind! =v 1
   ==>
   =goal>
   =imaginal>
   value busy
   color =v
   slot4 free)
|#

  ;; only p1 rhs dynamic
  
   (p c140
     =imaginal>
     test-num 140
     value nil
     color nil
     
     =goal>
     slot1 =s1
     
     !safe-bind! =v 1
     ==>
     =goal>
     =imaginal>
     =s1 =v
     value t)
  
  (p d140
     =imaginal>
     test-num 140
     slot4 nil
     value t
     =goal>
     slot4 =s4
     ==>
     =goal>
     =imaginal>
     slot4 free
     value busy)

#|
(p c140+d140
   =imaginal>
   test-num 140
   value nil
   color     nil
   slot4     nil
   =goal>
    slot1 =s1
   slot4 =s4
   !safe-bind! =v 1
   ==>
   =goal>
   =imaginal>
   value busy
   =s1 =v
   slot4 free)
|#

    
    ;; only p2 lhs dynamic
  
   (p c141
     =imaginal>
     test-num 141
     value nil
     color nil
     
     =goal>
     slot1 =s1
     
     !safe-bind! =v 1
     ==>
     =goal>
     =imaginal>
     color =v
     value t)
  
  (p d141
     =imaginal>
     test-num 141
     =s4 nil
     value t
     =goal>
     slot4 =s4
     ==>
     =goal>
     =imaginal>
     slot4 free
     value busy)
   
#|
(p c141+d141
   =imaginal>
   test-num 141
   value nil
   color     nil
   =s4     nil
   =goal>
    slot1 =s1
   slot4 =s4
   !safe-bind! =v 1
   ==>
   =goal>
   =imaginal>
   value busy
   color =v
   slot4 free)
|#

 
    ;; only p2 rhs dynamic
  
    (p c142
     =imaginal>
     test-num 142
     value nil
     color nil
     
     =goal>
     slot1 =s1
     
     !safe-bind! =v 1
     ==>
     =goal>
     =imaginal>
     color =v
     value t)
  
  (p d142
     =imaginal>
     test-num 142
     slot4 nil
     value t
     =goal>
     slot4 =s4
     ==>
     =goal>
     =imaginal>
     =s4 free
     value busy)
  
  
#|
(p c142+d142
   =imaginal>
   test-num 142
   value nil
   color     nil
   slot4     nil
   =goal>
    slot1 =s1
   slot4 =s4
   !safe-bind! =v 1
   ==>
   =goal>
   =imaginal>
   value busy
   color =v
   =s4 free)
|#
  
  
  ;;; test dynamic slot from a bind variable
  
  
  
    (p c143
     =imaginal>
     test-num 143
     value nil
     =s1 nil
     
     =goal>
     slot1 =s1
     
       !safe-bind! =v 1
       !safe-bind! =s2 'color
     ==>
     =goal>
     =imaginal>
     =s2 =v
     value t)
  
  (p d143
     =imaginal>
     test-num 143
     =s1 =x
     value t
     
     =goal>
     slot1 =s1

     !safe-bind! =s2 'value
     ==>
     =goal>
     =imaginal>
     =s1 free
     =s2 busy)
  

#|
   same issue as a & b 143 with 
   =s2 maybe needing instantiated somehow 
   since it's important in the mapping of
   the action

(p c143+d143
   =imaginal>
   test-num 143
   value nil
   =s1 nil
   =goal>
   slot1 =s1
   !safe-bind! =v 1
   !safe-bind! =s2 'color
   !safe-bind! =s3 'value
   ==>
   =goal>
   =imaginal>
   =s3 busy
   =s1 free)
|#



;;; This is case 143 where in c =s1 is not =s2 and a different var occurs in d

  
    (p c144
     =imaginal>
     test-num 144
     value nil
     =s1 nil
     
     =goal>
     slot1 =s1
     
       !safe-bind! =v 1
       !safe-bind! =s2 'slot4
     ==>
     =goal>
     =imaginal>
     =s2 =v
     value t)
  
  (p d144
     =imaginal>
     test-num 144
     =s4 =x
     value t
     
     =goal>
     slot4 =s4
     slot1 =s1
     !safe-bind! =s2 'value
     ==>
     =goal>
     =imaginal>
     =s1 free
     =s2 busy)
  

#|
(p c144+d144
   =imaginal>
   test-num 144
   value nil
   =s1 nil
   
   =goal>
   slot1 =s1
   slot4 =s4
   !safe-bind! =v 1
   !safe-bind! =s2 'slot4
   !safe-bind! =s3 'value
   ==>
   =goal>
   =imaginal>
   =s2 =v
   =s3 busy
   =s1 free)
|#


    
     (p c145
       =imaginal>
       test-num 145
       value nil
       !safe-bind! =v 'value
       ==>
       =imaginal>
       value t)

     (p d145
       =imaginal>
        value t
       test-num 145
       !safe-bind! =v 'value
       ==>
       =imaginal>
        =v 3)

#|

(p c145+d145
   =imaginal>
   test-num 145
   value nil
   !safe-bind! =v 'value
   ==>
   =imaginal>
   =v 3)

|#

   (p c146
     =imaginal>
     test-num 146
     value nil
     !safe-bind! =v 'value
     ==>
     =imaginal>
      value t
      color blue)

  (p d146
    =imaginal>
     test-num 146
     color blue
    !safe-bind! =v 'value
   ==>
    =imaginal>
     color red
      =v 3)

#|

(p c146+d146
   =imaginal>
   test-num 146
   value nil
   !safe-bind! =v 'value
   ==>
   =imaginal>
   color red
   =v 3)

|#

  ;; change the value of a slot from which a dynamic
  ;; is used and use that new value in p2

  (p e1
     =goal>
     test-num 200
     value nil
     =imaginal>
     slot1 =s
     !safe-bind! =v 1
     ==>
     =imaginal>
     slot1 value
     =goal>
     =s =v
     value t)
  
  (p f1
     =goal>
     test-num 200
     =s t
     =imaginal>
     slot1 =s
     ==>
     =imaginal>
     =goal>
     value busy)


#|

(p e1&f1
   =goal>
    test-num 200
    value nil
   =imaginal>
    slot1 =s
   !safe-bind! =v 1
   ==>
   =imaginal>
    slot1 value
   =goal>
    =s =v
    value busy)

|#   

  ;; do that with a variable instead of constant

 (p e2
     =goal>
     test-num 201
     value nil
     =imaginal>
     slot1 =s
     slot3 =s3
     !safe-bind! =v 1
     ==>
     =imaginal>
     slot1 =s3
     =goal>
     =s =v
     value t)
 
  (p f2
     =goal>
     test-num 201
     =s t
     =imaginal>
     slot1 =s
     ==>
     =imaginal>
     =goal>
     value busy)


#|

(p e2&f2
   =goal>
    test-num 201
    value nil
   =imaginal>
    slot1 =s
    slot3 value
   !safe-bind! =v 1
   ==>
   =imaginal>
    slot1 value
   =goal>
    =s =v
    value busy)

|#   

)