(clear-all)

(define-model child-slots
   (sgp :act t :style-warnings nil)
  
  (chunk-type parent)
  (chunk-type (target (:include parent)) slot1)
  (chunk-type (other (:include parent)) slot2)
  
  (chunk-type test count)
  
  (add-dm (parent isa parent)
          (target-nil isa target)
          (target-full isa target slot1 t)
          (other isa other slot2 t))
  
  (set-buffer-chunk 'imaginal (car (define-chunks (slot1 isa target slot1 slot1))))
  
  
  (defun matching ()
    (clear-productions)
    
    (p slot1-empty
       =goal>
       isa parent
       slot1 nil
       ==>
       )
    
    (p slot1-filled
       =goal>
       isa parent
       slot1 =val
       ==>
       )
    
    (p slot1-filled*
        =goal>
        isa parent
        =slot =val
        =imaginal>
        isa target
        slot1 =slot
        ==>
        ))
  
  (defun retrieving ()
    (clear-productions)
    
    (p retrieve0
       =goal>
       isa test
       count 0
       ==>
       -goal>
       +retrieval>
       isa target 
       slot1 nil)
    
    (p retrieve1
       =goal>
       isa test
       count 1
       ==>
       -goal>
       +retrieval>
       isa parent 
       slot1 nil)
    
    
    (p retrieve2
       =goal>
       isa test
       count 2
       ==>
       -goal>
       +retrieval>
       isa target
       - slot1 nil)
    
    
    (p retrieve3
       =goal>
       isa test
       count 3
       ==>
       -goal>
       +retrieval>
       isa parent 
       - slot1 nil)
    
    (p retrieve4
       =goal>
       isa test
       count 4
       ==>
       -goal>
       +retrieval>
       isa target
       slot1 t)
    
    
    (p retrieve5
       =goal>
       isa test
       count 5
       ==>
       -goal>
       +retrieval>
       isa parent 
       slot1 t)
    
    (p retrieve6
        =goal>
        isa test
        count 6
        =imaginal>
        isa target
        slot1 =slot
        
        ==>
        -goal>
        =imaginal>
        +retrieval>
        isa parent 
        =slot t)
    
    (p retrieve7
        =goal>
        isa test
        count 7
        =imaginal>
        isa target
        slot1 =slot
        
        ==>
        -goal>
        =imaginal>
        +retrieval>
        isa target 
        =slot t)
    
    (p retrieve8
        =goal>
        isa test
        count 8
        =imaginal>
        isa target
        slot1 =slot
        
        ==>
        -goal>  
        =imaginal>
        +retrieval>
        isa other 
        =slot t)
    )
  
  (defun imaginal ()
    (clear-productions)
    
    (p imaginal0
       =goal>
       isa test
       count 0
       
       ==>
       -goal>  
       +imaginal>
       isa parent 
       slot1 t)
    
    (p imaginal1
       =goal>
       isa test
       count 1
       
       ==>
       -goal>  
       +imaginal>
       isa target 
       slot1 t)
    
    
    (p imaginal2
        =goal>
        isa test
        count 2
        =imaginal>
        isa target
        slot1 =slot
        
        ==>
        -goal>  
        +imaginal>
        isa parent 
        =slot t)
    
    (p imaginal3
        =goal>
        isa test
        count 3
        =imaginal>
        isa target
        slot1 =slot
        
        ==>
        -goal>  
        +imaginal>
        isa target 
        =slot t)
    
    (p imaginal4
        =goal>
        isa test
        count 4
        =imaginal>
        isa target
        slot1 =slot
        
        ==>
        -goal>  
        +imaginal>
        isa other 
        =slot t)
    
    (p imaginal-bad
       =goal>
       isa test
       count nil
       
       ==>
       -goal>  
       +imaginal>
       isa other
       slot1 t)
    
    
    ))

