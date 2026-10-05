(clear-all)

(define-model audio-updates
    
  (sgp :unstuff-aural-location .5)
  
  (chunk-type (special-audio (:include audio-event)) only-evt both)
  (chunk-type (special-sound (:include sound)) only-sound both)
                               
  
  (set-parameter-value :audio-ms-times *audio-ms-times*)
  
  (p stuffed
     ?goal>
     buffer empty
     =aural-location>
     ==>
     !eval! (pprint-chunks-fct (list =aural-location))
     +goal>
     =aural-location>
     )
  
  (p unstuffed
     =goal>
     value nil
     ?aural-location>
     state free
     buffer empty
     ==>
     =goal>
     value 1
     +aural-location>
     isa audio-event
     offset highest
     !stop!)
  
  (p result
     =goal>
     value 1
     =aural-location>
     ==>
     !eval! (pprint-chunks-fct (list =aural-location))
     =goal>
     value 2)
  
  (p find-an-ongoing
     =goal>
     value 2
     ==>
     +aural-location>
     isa audio-event
     kind speech
     =goal>
     value 3)
  
  (p before-ongoing-stopped
     =goal>
     value 3
     =aural-location>
     ==>
     !eval! (pprint-chunks-fct (list =aural-location))
     =aural-location>
     =goal>
     value 4
     !stop!)
  
  (p after-ongoing-stopped
     =goal>
     value 4
     =aural-location>
     ==>
     !eval! (pprint-chunks-fct (list =aural-location))
     +aural>
     isa sound
     event =aural-location
     =goal>
     value 5)
  
  (p found-stopped-ongoing
     =goal>
     value 5
     =aural>
     event =e
     ==>
     !eval! (pprint-chunks-fct (list =aural =e))
     =goal>
     value 6
     +aural-location>
     isa audio-event
     kind word
     )
  (p attend-other
     =goal>
     value 6
     =aural-location>
     ==>
     !eval! (pprint-chunks-fct (list =aural-location))
     +aural>
     isa sound
     event =aural-location
     =goal>
     value 7
     !stop!)
  
  (p ongoing-stoped-before-aural-completion
     =goal>
     value 7
     =aural>
     event =e
     ==>
     !eval! (pprint-chunks-fct (list =aural =e))
     =goal>
     value 8
     )
  
)

     