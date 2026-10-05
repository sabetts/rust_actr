
;; Custom functions to send output to *out-file* and set a seed.



(actr-load "ACT-R:tutorial;lisp;zbrodoff.lisp")

(defun run-it ()
  (reset)
  (sgp :seed (100 0))
  (sgp-fct (list :v *out-file*))
  (model-output "? (zbrodoff-set)")
  (sgp :v nil)
  (zbrodoff-set)
  (sgp-fct (list :v *out-file*))
  (model-output "~%>>> zbrodoff.block()") ;fake running the python version
  (sgp :v nil)
  (zbrodoff-block))
  
  

(defun print-analysis (rts counts blocks addends totals)
  (format *out-file* "~%        ")
  (dotimes (addend (length addends))
    (format *out-file* " ~6@a (~2d)" (nth addend addends) (nth addend totals)))
  (dotimes (block (length blocks))
    (format *out-file* "~%Block ~2d" (nth block blocks))
    (dotimes (addend (length addends))
      (format *out-file* " ~6,3f (~2d)" (nth (+ addend (* block (length addends))) rts)
        (nth (+ addend (* block (length addends))) counts))))
  (terpri *out-file*))
