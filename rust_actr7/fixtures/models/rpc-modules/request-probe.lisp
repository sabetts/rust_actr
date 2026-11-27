(clear-all)

(define-model request-probe
  (sgp :v nil)
  (chunk-type probe state)
  (p request
    ?probe> state free
    ==>
    +probe> isa probe state ready))
