
(ssp :starting-parameters (:seed (1 0) :testing-function set-testing-traces))

(defun run-it ()
  (model-output "? (fan-sentence \"lawyer\" \"store\" t 'person)")
  (model-output ">>> fan.sentence('lawyer','store',True,'person')")
  (model-output "")
  (fan-sentence "lawyer" "store" t 'person))


(load-act-r-code "ACT-R:tutorial;lisp;fan.lisp")
