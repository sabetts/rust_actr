
(defvar *act-trace* nil)
(defvar *compilation* nil)

(ssp :starting-parameters (:testing-function set-paired-params))

(defun set-paired-params ()
  (set-testing-traces)
  (sgp-fct (list :act *act-trace* :epl *compilation* :pct *compilation*)))
    
(defun run-it ()
  (record-history "buffer-trace")
  (record-history "goal" "visual" "visual-location" "imaginal" "retrieval" "manual")

  (setf *act-trace* t *compilation* nil)
  (paired-task 1 2)
  (sdm isa pair)
  (setf *act-trace* nil *compilation* t)
  (paired-task 1 2)
  (setf *act-trace* nil *compilation* nil)
  (paired-task 2 2)
  (predict-bold-response))


(load-act-r-code "ACT-R:tutorial;lisp;paired.lisp")
