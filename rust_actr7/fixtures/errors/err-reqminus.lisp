(clear-all)

(define-model err-reqminus

(sgp :esc t :lf .05 :trace-detail high)

(chunk-type number number next)
(chunk-type count-from start end count)

(add-dm
 (one ISA number number one next two)
 (two ISA number number two next three)
 (three ISA number number three next four)
 (four ISA number number four next five)
 (five ISA number number five))

(goal-focus (ISA count-from start two end four))

(p start
   =goal>
      ISA         count-from
      start       =num1
      count       nil
 ==>
   =goal>
      ISA         count-from
      count       =num1
   +retrieval>
      ISA         number
      number      =num1
      - bogus-slot bogus-value
   )
)
