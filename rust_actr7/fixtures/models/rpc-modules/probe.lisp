(clear-all)

(define-model probe-model
  (sgp :v nil)
  (p probe
    ?probe> state free
    ==>
    !output! (probe-fired)))
