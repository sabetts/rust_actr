
(clear-all)

(define-model remove-and-restored

(sgp :seed (123456 0) :unstuff-visual-location nil)
  
(chunk-type read-letters state)
(chunk-type array letter)

(add-dm 
 (start isa chunk) (attend isa chunk)
 (respond isa chunk) (done isa chunk)
 (goal isa read-letters state start))

(P find-unattended-letter
   =goal>
      ISA         read-letters
      state       start
 ==>
   +visual-location>
      ISA         visual-location
      :attended    nil
      screen-x lowest
   =goal>
      state       find-location
)

(P attend-letter
   =goal>
      ISA         read-letters
      state       find-location
   =visual-location>
   ?visual>
      state       free
   
==>
   +visual>
      ISA         move-attention
      screen-pos  =visual-location
   =goal>
      state       attend
)

(P encode-letter
   =goal>
      ISA         read-letters
      state       attend
   =visual>
      ISA         text
      value       =letter
   ?imaginal>
      state         free
==>
   =goal>
      state       start
   +imaginal>
      isa         array
      letter      =letter
)


(P respond
   =goal>
      ISA         read-letters
      state       find-location
   =imaginal>
     isa array
     letter =letter
   ?visual-location>
      state       error
   ?manual>   
      state       free
==>
   =goal>
      state       done
   +manual>
      ISA         press-key
      key         =letter
)

(goal-focus goal)

)
