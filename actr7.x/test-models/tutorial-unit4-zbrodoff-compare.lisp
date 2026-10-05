
;; Custom functions to send output to *out-file* and set a seed.



(actr-load "ACT-R:tutorial;lisp;zbrodoff.lisp")

(defun run-it ()
  (sgp-fct (list :v *out-file*))
  (model-output "? (zbrodoff-compare 10)")
  (model-output "~%>>> zbrodoff.compare(10)~%")
  (sgp :v nil)
  (zbrodoff-compare 1))
  

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

(defun zbrodoff-compare (n)
  (let ((results nil))
    (dotimes (i n)
      (push (zbrodoff-experiment nil nil) results))
    
    (let ((rts (mapcar (lambda (x) (/ x (length results)))
                 (apply 'mapcar '+ (mapcar 'first results))))
          (counts (mapcar (lambda (x) (truncate x (length results)))
                    (apply 'mapcar '+ (mapcar 'second results)))))
      
      (correlation rts *zbrodoff-control-data* *out-file*)
      (mean-deviation rts *zbrodoff-control-data*)
      
      (print-analysis rts counts '(1 2 3) '("2" "3" "4") '(64 64 64)))))

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
