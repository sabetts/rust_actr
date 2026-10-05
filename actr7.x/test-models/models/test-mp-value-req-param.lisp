
(clear-all)
(define-model test-mp-value-param
  (sgp-fct (list :esc t :act t :rt -9 :mp 10 :trace-detail 'high))
  
  (chunk-type test slot)
  
  (add-dm (a isa test slot a)
          (b isa test slot b)
          (c isa test slot c))
  
  (set-similarities (a a -.9) (a c -.1) (a b -.5))
  
  (p p1
     ?goal>
     buffer empty
     ==>
     +goal>
     isa test
     slot 2
     +retrieval>
     isa test
     slot a)
  
  (p p2
     =goal>
     isa test
     slot 2
     =retrieval>
     ==>
     =goal>
     slot 3
     +retrieval>
     isa test
     :mp-value nil
     slot a)
  
  (p p3
     =goal>
     isa test
     slot 3
     =retrieval>
     ==>
     =goal>
     slot 4
     +retrieval>
     isa test
     :mp-value 3
     slot c)
  
  (p p4
     =goal>
     isa test
     slot 4
     =retrieval>
     ==>
     =goal>
     slot 5
     +retrieval>
     isa test
     slot b)
  
  (p p5
     =goal>
     isa test
     slot 5
     =retrieval>
     ==>
     =goal>
     slot 6
     +retrieval> (b :mp-value 2)
     )
  
  (p p6
     =goal>
     isa test
     slot 6
     =retrieval>
     ==>
     =goal>
     slot 7
     +retrieval>
     isa test
     :mp-value t
     )
  (p p7
     =goal>
     isa test
     slot 7
     ==>
     =goal>
     slot 8
     +retrieval>
     isa test
     :mp-value 3
     :mp-value nil
     )
  (p p8
     =goal>
      isa test
      slot 8
    ==>
     =goal>
     slot 9
     +retrieval>
     isa test
     - :mp-value nil
     ))
