
(ssp :starting-parameters (:seed (1 0) :sact t :testing-function set-testing-traces))


(defun run-it ()
  (let ((*standard-output* *out-file*))
    (fan-sentence "lawyer" "store" t 'person)
    (print-chunk-activation-trace p13 585)
    (print-chunk-activation-trace p1 585)
    (fan-sentence "hippie" "bank" t 'person)
    (print-activation-trace 585)))

(load-act-r-code "ACT-R:tutorial;lisp;fan.lisp")
