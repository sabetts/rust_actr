(:NAME "tutorial-unit6-chunks-with-default-slots" :REF "tutorial-unit6-chunks-with-default-slots.ref" :MODELS ("test-models;models;unit6-chunk-type-default-slots.lisp") :SEED-SET NIL :RUNS NIL :STEPS ((:MODEL-OUTPUT "? (define-chunks
   (c1 isa a)
   (c2 isa b)
   (c3 isa b slot 3)
   (c4 isa c)
   (c5 isa c slot2 nil))") (:MODEL-OUTPUT "(C1 C2 C3 C4 C5)") (:MODEL-OUTPUT "
? (pprint-chunks c1 c2 c3 c4 c5)") (:COMMAND PPRINT-CHUNKS (C1 C2 C3 C4 C5)) (:MODEL-OUTPUT "(C1 C2 C3 C4 C5)")) :GOAL-FOCUS 0 :RESETS 0 :SGP NIL :SCOPE :COMMAND :FLAGS NIL :ERROR NIL)
