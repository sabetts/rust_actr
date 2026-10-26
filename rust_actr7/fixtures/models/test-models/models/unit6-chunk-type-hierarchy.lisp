(clear-all)
(define-model isa-test
    
  (chunk-type a slot1)
  (chunk-type b slot2)
  (chunk-type (c (:include a)) slot3)
  (chunk-type (d (:include a) (:include b)) slot4)
  
  )

;(chunk-type a slot1)
;(chunk-type b slot2)
;(chunk-type c slot1 slot3)
;(chunk-type d slot1 slot2 slot4)