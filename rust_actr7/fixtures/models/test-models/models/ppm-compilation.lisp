

(clear-all)

(define-model test-ppm-compilation
    
    (sgp :esc t :ppm 1.0 :epl t :pct t)
  
  (add-dm (small isa chunk)
          (medium isa chunk)
          (large isa chunk))
  
  (define-chunks (p1)(p2)(p3)(p4)(p5)(p6)(p7)(p8)(p9)
    (p10)(p11)(p12)(p13)(p14)(p15)(p16)(p17)(p18)(p19)
    (p20)(p21)(p22)(p23)(p24)(p25))
  
  (set-similarities (small medium -.5) (medium large -.5) (small large -.9))
  
  (chunk-type test slot small med large state)
  
  (goal-focus-fct (car (define-chunks (isa test slot medium small small med medium large large))))
  
  
  ; test all possible cases for a single slot in a goal style buffer:
  ; constant, unbound var, bound var in each of p1 and p2 
  ; for each pair can be a mismatch to: first, second, both same, or both different except in the unbound case which can't mismatch
  ;(assume both match works since that doesn't involve a partial match)
  
  ; - constant in p1 and p2 mismatch p1 (p0 & p1)
  ; - constant in p1 and p2 mismatch p2 (p1 & p2)
  ; - constant in p1 and p2 mismatch both same (P2 & p3)
  ; - constant in p1 and p2 mismatch both different (p3 & p4)
  
  ; - constant in p1, unbound var in p2, mismatch to p1  (p4 & p5)
  ; - unbound var in p1, constant in p2, mismatch to p2 (p5 & p6)
  
  ; - constant in p1, bound var in p2, mismatch to p1 (p6 & p7)
  ; - bound var in p1, constant in p2, mismatch to p2 (p7 & p8)
  ; - constant in p1, bound var in p2, mismatch same (p8 & p9)
  ; - bound var in p1, constant in p2, mismatch same (p9 & p10)
  ; - constant in p1, bound var in p2, mismatch different (p10 & p11)
  ; - bound var in p1, constant in p2, mismatch different (p11 & p12)
  
  ; - constant in p1, bound var in p2, mismatch p2 (p13 & p14)
  ; - bound var in p1, constant in p2, mismatch in p1 (p14 & p15)
  
  ; - unbound var in p1, bound var in p2, mismatch in p2 (p16 & p17)
  
  ; - bound var in p1, bound var in p2, mismatch in p1 (p17 & p18)
  ; - bound var in p1, bound var in p2, mismatch in p2 (p18 & p19)
  ; - bound var in p1, bound var in p2, mismatch both same (p19 & p20)
  ; - bound var in p1, bound var in p2, mismatch both different (p20 & p21)
  
  ; - bound var in p1, unbound var in p2, mismatch in p1 (p21 & p22)

  
  
  (p p0 
     =goal>
     isa test
     slot small
     state nil
     ==>
     =goal>
     state p1)
  
  (p p1
     =goal>
     isa test
     slot medium
     state p1
     ==>
     =goal>
     state p2)

  (p p2
     =goal>
     isa test
     slot small
     state p2
     ==>
     =goal>
     state p3)
  
  (p p3
     =goal>
     isa test
     slot small
     state p3
     ==>
     =goal>
     state p4)
     
  (p p4
     =goal>
     isa test
     slot large
     state p4
     ==>
     =goal>
     state p5)
  
  (p p5
     =goal>
     isa test 
     slot =s
     state p5
     ==>
     =goal>
     state p6)
  
  (p p6
     =goal>
     isa test 
     slot small
     state p6
     ==>
     =goal>
     state p7)
  
  (p p7
     =goal>
     isa test 
     slot =med
     med =med
     state p7
     ==>
     =goal>
     state p8)
  
  (p p8
     =goal>
     isa test 
     slot small
     state p8
     ==>
     =goal>
     state p9)
  
  (p p9
     =goal>
     isa test 
     small =small
     slot =small
     state p9
     ==>
     =goal>
     state p10)
  
  (p p10
     =goal>
     isa test 
     slot small
     state p10
     ==>
     =goal>
     state p11)
  
  (p p11
     =goal>
     isa test 
     slot =large
     large =large
     state p11
     ==>
     =goal>
     state p12)
  
  (p p12
     =goal>
     isa test 
     slot small
     state p12
     ==>
     =goal>
     state p13)
  
  (p p13
     =goal>
     isa test
     slot medium
     state p13
     ==>
     =goal>
     state p14)
  
  (p p14
     =goal>
     isa test 
     slot =large
     large =large
     state p14
     ==>
     =goal>
     state p15)
  
  (p p15
     =goal>
     isa test
     slot medium
     state p15
     ==>
     =goal>
     state p16)
  
  (p p16
     =goal>
     isa test 
     slot =s
     state p16
     ==>
     =goal>
     state p17)
  
  (p p17
     =goal>
     isa test 
     slot =large
     large =large
     state p17
     ==>
     =goal>
     state p18)
  
  (p p18
     =goal>
     isa test 
     slot =med
     med =med
     state p18
     ==>
     =goal>
     state p19)
  
    (p p19
     =goal>
     isa test 
     slot =large
     large =large
     state p19
     ==>
     =goal>
       state p20)
  
  (p p20
     =goal>
     isa test 
     slot =large
     large =large
     state p20
     ==>
     =goal>
     state p21)
  
  (p p21
     =goal>
     isa test 
     slot =small
     small =small
     state p21
     ==>
     =goal>
     state p22)
  
  (p p22
     =goal>
     isa test 
     slot =s
     state p22
     ==>
     =goal>
     state p23)
  )
