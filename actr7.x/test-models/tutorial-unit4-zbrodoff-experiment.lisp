
;; Custom functions to send output to *out-file* and set a seed.



(actr-load "ACT-R:tutorial;lisp;zbrodoff.lisp")

(defun run-it ()
  (sgp-fct (list :v *out-file*))
  (model-output "? (zbrodoff-experiment)")
  (model-output "~%>>> zbrodoff.experiment()~%")
  (sgp :v nil)
  (zbrodoff-experiment))
  

(defun zbrodoff-experiment (&optional (visible (not *run-model*)) (show t))
  (reset)
  (sgp :v nil :seed (100 0))
  (setf *trials* nil)
  (dotimes (j 3)
    (dotimes (i 8)
      (setf *trials* (append *trials* (create-set (+ j 1) visible)))))
  (setf *results* nil)
  (collect-responses)
  (analyze-results show))

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
