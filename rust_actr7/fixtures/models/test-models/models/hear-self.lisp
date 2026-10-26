(clear-all)

(define-model test-hear-self
    (p speak
       ?goal>
         buffer empty
       ?vocal>
         state free
       ==>
       +vocal>
         isa speak
         string "Hello"
       +goal>)
  
  (p subvocalize
     =goal>
       state nil
     ?vocal>
       state free
     ==>
     +vocal>
       isa subvocalize
       string "Goodbye"
     =goal>
       state t))
