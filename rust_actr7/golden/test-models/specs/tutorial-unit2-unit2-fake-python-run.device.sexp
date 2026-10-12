; Hand-authored: the task draws its three letters at random (`act-r-random`
; before the run), so the extractor's auto-generated window content churns
; between regens. The ref is letter-independent (the trace only shows the
; visual-location churn), so any fixed letters reproduce it.
(RUN 1
 (ADD-TEXT 0 "K" 125 75)
 (ADD-TEXT 0 "D" 75 175)
 (ADD-TEXT 0 "D" 175 175)
)
