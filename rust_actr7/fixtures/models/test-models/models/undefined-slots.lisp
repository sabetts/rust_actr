(clear-all)

(define-model reset-slot-order-test
    (chunk-type oops slot1)
    (p a
       =goal>
       isa oops
       state t
       slot1 =x
       slot2 =y
       slot4 =z
       ==>
       )
  (p b
     =goal>
     state nil
     slot2 t
     ==>
     ))
