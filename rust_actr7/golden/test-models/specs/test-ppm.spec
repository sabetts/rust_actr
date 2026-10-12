(:NAME "test-ppm" :REF "test-ppm.ref" :MODELS ("test-models;models;test-ppm.lisp") :SEED-SET NIL :RUNS ((10 (100 0) T 22) (10 (100 0) T 22) (10 (100 0) T 22)) :STEPS ((:RESET) (:SGP :PPM NIL) (:MODEL-OUTPUT "
******************************
Without ppm (default symbolic matching):
******************************
") (:COMMAND WHYNOT NIL) (:SGP :ACT NIL :EPL NIL :PCT NIL :ALPHA 0.2) (:RUN 10 (100 0) T 22) (:RESET) (:SGP :PPM 1) (:SGP :UTILITY-OFFSETS PPM-OFFSET) (:MODEL-OUTPUT "
******************************
With ppm enabled:
******************************
") (:COMMAND WHYNOT NIL) (:RUN 10 (100 0) T 22) (:RESET) (:SGP :PPM 1 :PPM-HOOK SHOW-PPM-VALUES) (:SGP :UTILITY-OFFSETS PPM-OFFSET) (:MODEL-OUTPUT "
******************************
With ppm enabled and using the hook to show values as it goes:
******************************
") (:RUN 10 (100 0) T 22)) :GOAL-FOCUS 0 :RESETS 3 :SGP ((:PPM NIL) (:PPM 1) (:UTILITY-OFFSETS PPM-OFFSET) (:PPM 1 :PPM-HOOK SHOW-PPM-VALUES) (:UTILITY-OFFSETS PPM-OFFSET)) :SCOPE :MULTI :FLAGS NIL :ERROR NIL)
