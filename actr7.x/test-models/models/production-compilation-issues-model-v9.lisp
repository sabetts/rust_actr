(clear-all)


(define-model compilation-test
    (sgp :seed (79246602791 15065))
  (sgp :esc t :lf .5 :bll .5 :mp 18 :rt -3 :ans .25)
  (sgp :style-warnings nil)
  (sgp :v nil :trace-detail low)
  (sgp :ul t :egs 1.0 :iu 15)
  (sgp :epl t :pct t :tt 10)
  
  (chunk-type task state)
  (chunk-type number visual-rep representation)
  (chunk-type response key (is-response t))
  (chunk-type trial num1 num2 result response)
         
  (define-chunks (win isa chunk)
    (draw isa chunk)
    (lose isa chunk)
    (attend-num-1 isa chunk)
    (encode-num-1 isa chunk)
    (find-num-2 isa chunk)
    (attend-num-2 isa chunk)
    (encode-num-2 isa chunk)
    (num2 isa chunk)
    (retrieve-past-trial isa chunk)
    (process-past-trial isa chunk)
    (respond isa chunk)
    (guess-other isa chunk)
    (detect-feedback isa chunk)
    (encode-feedback isa chunk))
  
  (add-dm (zero isa number visual-rep "0" representation zero)
          (one isa number visual-rep "1" representation one)
          (two isa number visual-rep "2" representation two)
          (three isa number visual-rep "3" representation three))
          
  (add-dm (response-1 isa response key "s")
          (response-2 isa response key "d")
          (response-3 isa response key "f"))

  
  (set-similarities (win draw -.2) (win lose -.15)
                    (zero one -.1) (one two -.1) (two three -.1) 
                    (zero two -.2) (one three -.2) 
                    (zero three -.3))
  
  (set-all-base-levels 1000 -100)
  
  (goal-focus-fct (car (define-chunks (isa task))))
  
  (p detect-trial-start
     =goal>
       isa task
       state nil
     =visual-location>
       isa visual-location
     ?visual>
       state free
     ?imaginal>
       state free
     ==>
     =goal>
       state attend-num-1
     +imaginal>
       isa trial
     +visual>
       cmd move-attention
       screen-pos =visual-location)
  
  (p attend-num-1
     =goal>
       isa task
       state attend-num-1
     =visual>
       isa visual-object
       value =val
     ==>
     =goal>
       state encode-num-1
     +retrieval>
       isa number 
       visual-rep =val)
  
  (p encode-num-1
     =goal>
       isa task
       state encode-num-1
     =retrieval>
       isa number
       representation =number
     =imaginal>
       isa trial
     ==>
     =imaginal>
       num1 =number
     =goal>
       state find-num-2
     +visual-location>
       isa visual-location
       > screen-x current
       :attended nil)
  
  (p find-num-2
     =goal>
       isa task
       state find-num-2
     =visual-location>
       isa visual-location
     ?visual>
       state free
     ==>
     =goal>
       state attend-num-2
     +visual>
       cmd move-attention
       screen-pos =visual-location)
  
  (p attend-num-2
     =goal>
       isa task
       state attend-num-2
     =visual>
       isa visual-object
       value =val
     ==>
     =goal>
       state encode-num-2
     +retrieval>
       isa number 
       visual-rep =val)
  
  (p encode-num-2
     =goal>
       isa task
       state encode-num-2
     =retrieval>
       isa number
       representation =number
     =imaginal>
       isa trial
     ==>
     =imaginal>
       num2 =number
     =goal>
       state retrieve-past-trial)
  
  
  (p retrieve-past-trial
     =goal>
       isa task
       state retrieve-past-trial
     =imaginal>
       isa trial
       num1 =n1
       num2 =n2
     ==>
     =imaginal>
     +retrieval>
       isa trial
       num1 =n1
       num2 =n2
       result win
     =goal>
       state process-past-trial)
  
  (p no-past-trial
     =goal>
       isa task 
       state process-past-trial
     ?retrieval>
       buffer failure
     ==>
     +retrieval>
       isa response
     =goal>
       state respond)
  
  (p retrieved-a-win
     =goal>
       isa task
       state process-past-trial
     =retrieval>
       isa trial
       result win
       response =response
     ==>
     +retrieval> 
       isa response
       key =response
     =goal>
       state respond)
  
  (p retrieved-a-non-win
     =goal>
       isa task
       state process-past-trial
     =retrieval>
       isa trial
      - result win
       response =response
     ==>
     +retrieval> 
       isa response
       key =response
     =goal>
       state guess-other)
  
  (p guess-other
     =goal>
       isa task
       state guess-other
     =retrieval>
       isa response
       key =key
     ==>
     +retrieval>
       isa response
      - key =key
     =goal>
       state respond)
  
  (p respond
     =goal>
       isa task
       state respond
     =retrieval>
       isa response
       key =key
     ?manual>
       state free
     =imaginal>
       isa trial     
     ==>
     =imaginal>
       response =key
     +manual>
       cmd press-key
       key =key
     =goal>
       state detect-feedback)
  
  (p respond-when-response-failure
     =goal>
       isa task
       state respond
     ?retrieval>
       buffer failure
     ?manual>
       state free
     =imaginal>
       isa trial
     ==>
     =imaginal>
       response response-2
     +manual>
       cmd press-key
       key "d"
     =goal>
       state detect-feedback)
  
  (p detect-feedback
     =goal>
       isa task
       state detect-feedback
     =visual-location>
       isa visual-location
     ?visual>
       state free
     ==>
     +visual>
       cmd move-attention
       screen-pos =visual-location
     =goal>
       state encode-feedback)
  
  (p encode-feedback-win
     =goal>
       isa task 
       state encode-feedback
     =imaginal>
       isa trial
     =visual>
       isa visual-object
       value "win"
     ?manual>
       state free
     ==>
     =imaginal>
       result win
     +manual>
       cmd press-key
       key space
     +goal>
       isa task)
     
  (p encode-feedback-lose
     =goal>
       isa task 
       state encode-feedback
     =imaginal>
       isa trial
     =visual>
       isa visual-object
       value "lose"
     ?manual>
       state free
     ==>
     =imaginal>
       result lose
     +manual>
       cmd press-key
       key space
     +goal>
       isa task)    
  
    (p encode-feedback-draw
     =goal>
       isa task 
       state encode-feedback
     =imaginal>
       isa trial
     =visual>
       isa visual-object
       value "draw"
     ?manual>
       state free
     ==>
     =imaginal>
       result draw
     +manual>
       cmd press-key
       key space
     +goal>
       isa task)
  
  (spp encode-feedback-win :reward 18)
  (spp encode-feedback-lose :reward 12)
  (spp encode-feedback-draw :reward 15)
  
)
