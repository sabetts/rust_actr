(clear-all)

(define-model compilation-tests
    
  (sgp :epl t :pct t :do-not-harvest imaginal :do-not-harvest visual-location) 
  (chunk-type test state color)
  
  (define-chunks (color) (x state color color blue)) 
  (goal-focus x) 
  (set-buffer-chunk 'visual-location 'x)   
  (set-buffer-chunk 'imaginal 'x)
  
  (declare-buffer-usage imaginal test :all)
  
  ;; test a goal, imaginal, and perceptual in the
  ;; simple cases (need do-not-harvest for 1 and not for 2)
  
  (p g1 =goal> state =s ==>)
  (p g2 =goal> state =s ==> =goal>)
  
  (p i1 =imaginal> state =s ==>)
  (p i2 =imaginal> state =s ==> =imaginal>)
  
  (p p1 =visual-location> state =s ==>)
  (p p2 =visual-location> state =s ==> =visual-location>)
  
  
  ;; with indirection -- just run with do-not-harvest set
  
  (p gd =goal> state =s =s =val ==>)
  (p id =imaginal> state =s =s =val ==>)
  (p pd =visual-location> state =s =s =val ==>)
  
  
  ;; with between buffer indirection again run with do-not-harvest set
  
  
  (p gi =goal> state =s =imaginal> =s =val ==>)
  (p gp =goal> state =s =visual-location> =s =val ==>)
  
  (p ig =imaginal> state =s =goal> =s =val ==>)
  (p ip =imaginal> state =s =visual-location> =s =val ==>)
  
  (p pg =visual-location> state =s =goal> =s =val ==>)
  (p pi =visual-location> state =s =imaginal> =s =val ==>)
  
  (pdisable-fct (all-productions))
  )
