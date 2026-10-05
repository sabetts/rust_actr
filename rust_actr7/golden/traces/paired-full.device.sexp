; Scripted window input for the paired-full golden (mirrors task file).
; CLEAR empties the window; END schedules the run-full-time dummy.
(CLEAR 5000)
(ADD-TEXT 5000 "0" 300 100)
(CLEAR 10000)
(ADD-TEXT 10000 "card" 100 200)
(ADD-TEXT 0 "bank" 100 100)
(CLEAR 15000)
(ADD-TEXT 15000 "1" 300 200)
(END 20000)
