; Hand-authored: the task's `remove-items-from-exp-window` /
; `add-items-to-exp-window` and its `print-visicon` calls happen inside the
; `output-key` monitored-command callback, which the extractor does not
; execute (the "no arbitrary Lisp" divergence). The ops below are the
; callback's effects, timed from the ref. `key-event-handler` prints and then
; removes the letter; `redisplay-item` (scheduled +0.15) prints again, re-adds
; the letter, and schedules a late print. Run 2 keys on "b" but the letter it
; captured (`*letter*`) is "a", which is why the restored item is
; `VISUAL-LOCATION2` at "a"'s coordinates.
(RUN 1
 (ADD-TEXT 0 "a" 125 150)
 (PRINT-VISICON 695)
 (REMOVE-TEXT 695 "a")
 (PRINT-VISICON 845)
 (ADD-TEXT 845 "a" 125 150)
 (PRINT-VISICON-LATE 845)
)
(RUN 2
 (ADD-TEXT 0 "a" 125 150)
 (ADD-TEXT 0 "b" 150 150)
 (CLEAR 0)
 (PRINT-VISICON 1135)
 (REMOVE-TEXT 1135 "a")
 (PRINT-VISICON 1285)
 (ADD-TEXT 1285 "a" 125 150)
 (PRINT-VISICON-LATE 1285)
)
