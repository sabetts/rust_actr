(clear-all)
(define-model imaginal-dynamic
  (sgp :epl t :pct t :trace-detail low)
  (chunk-type state step val)
  (chunk-type image slot1 slot2)
  (add-dm (g isa state step one val color)
          (im isa image slot1 color slot2 color))
  (goal-focus g)
  (set-buffer-chunk 'imaginal 'im)
  (p imaginal-a
     =goal>
       step one
     =imaginal>
       slot1 =s
     !safe-bind! =v 1
     ==>
     =goal>
       step two
     =imaginal>
       slot1 new)
  (p imaginal-b
     =goal>
       step two
     =imaginal>
       slot2 =s
     ==>
     =imaginal>
       slot2 =s
     =goal>
       step done))
