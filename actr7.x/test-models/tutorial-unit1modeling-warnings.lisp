
(defun run-it () 
  (define-model a (sgp :v t) (define-chunks (a value starting))
    (p test ==>))
  (module-request 'retrieval (define-chunk-spec isa chunk))
  (module-request 'retrieval (define-chunk-spec isa chunk))  
  (p test ==>))
