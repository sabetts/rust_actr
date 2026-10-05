(clear-all)

(define-model mp-test

(sgp :esc t :lf .05 :mp 4.0 :rt -1.0 :trace-detail high)

(chunk-type fto kind)
(chunk-type goal state)

(add-dm
 (aaa) (aab) (aac)
 (t1 ISA fto kind aaa)
 (t2 ISA fto kind aab))

(goal-focus (ISA goal state start))

(p retrieve
   =goal>
      ISA      goal
      state    start
 ==>
   =goal>
      state    done
   +retrieval>
      ISA      fto
      kind     aac
   )

(p done
   =goal>
      ISA      goal
      state    done
   =retrieval>
 ==>
   -goal>
   )
)
