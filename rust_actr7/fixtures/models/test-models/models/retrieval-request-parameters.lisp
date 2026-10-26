(clear-all)

(define-model retrieval-request-params
  (sgp :esc t :ga 1 :bll .5 :mas 3 :mp 1 :ans .5 :rt -1 :act t :seed (100 2))
  
  (chunk-type t1 s1 s2)
  
  (add-dm (c1) (c2) (t1 s1 c1 s2 c1) (t2 s1 c2 s2 c1) (g1 color c2))
  
  (set-similarities (c1 c2 -.5))
  
  (goal-focus g1)
  
  (p normal
     =goal>
      state nil
     ==>
     =goal>
      state t
     +retrieval>
      s1 c1
      s2 c1)
  
  (p change-threshold
     =goal>
      state nil
     ==>
     =goal>
      state t
     +retrieval>
      :rt-value 10
      s1 c1
      s2 c1)
  
  (p no-bll
     =goal>
      state nil
     ==>
     =goal>
      state t
     +retrieval>
      :bll-value nil
      s1 c1
      s2 c1)
  
  (p no-mas
     =goal>
      state nil
     ==>
     =goal>
      state t
     +retrieval>
      :mas-value nil
      s1 c1
      s2 c1)
  
  (p no-mp
     =goal>
      state nil
     ==>
     =goal>
      state t
     +retrieval>
      :mp-value nil
      s1 c1
      s2 c1)
  
  (p no-ans
     =goal>
      state nil
     ==>
     =goal>
      state t
     +retrieval>
      :ans-value nil
      s1 c1
      s2 c1)
  
  (pdisable normal change-threshold no-bll no-mas no-mp no-ans)
  )


    