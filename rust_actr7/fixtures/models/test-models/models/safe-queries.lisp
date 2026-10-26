(clear-all)

(define-model safe-queries
    
    (p a
       =goal>
       ==>
       +imaginal>)
    (p b
       =goal>
       =imaginal>
       ==>
       *imaginal>)
    (p c
       =goal>
       =imaginal>
       ==>
       *imaginal>
       +imaginal>)
  (p d
     =goal>
     ==>
     +visual>)
  (p e
     =goal>
     ==>
     +retrieval>)
  (p f 
     =goal>
     ==>
     +goal>
     *goal>)
  (p f
     =goal>
     ==>
     +visual>
     +visual-location>
     +manual>))