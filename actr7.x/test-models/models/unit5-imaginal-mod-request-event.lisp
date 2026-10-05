(clear-all)
(define-model imaginal-action1
    (sgp :v nil :trace-detail high)
  (chunk-type count-from start count)
  (set-buffer-chunk 'imaginal '(start 1))
  
  (p start
     =imaginal>
        ISA      count-from
        start    =num1
        count    nil
   ==>
     *imaginal>
        ISA      count-from
        count    =num1 
  ) 
  )