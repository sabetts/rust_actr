
(ssp :starting-parameters (:seed (1 0) :testing-function set-testing-traces))

(defun trace-fan-run ()
  (set-testing-traces)
  (sgp :act t))

(defun run-it ()
  (let ((*standard-output* *out-file*))
    (fan-sentence "lawyer" "store" t 'person)
    (sdm - arg1 nil - arg2 nil)
    (ssp :starting-parameters (:seed (1 0) :testing-function trace-fan-run))
    
    (fan-sentence "lawyer" "store" t 'person)
    (fan-sentence "hippie" "bank" t 'person)
    (fan-sentence "hippie" "bank" t 'location)
    (fan-sentence "giant" "bank" t 'person)
    (fan-sentence "giant" "bank" t 'location)
    (ssp :starting-parameters (:seed (1 0) :testing-function set-testing-traces))
    (fan-experiment)))

(load-act-r-code "ACT-R:tutorial;lisp;fan-no-pm.lisp")
