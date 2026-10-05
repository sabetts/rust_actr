


;; Custom functions to send output to *out-file* and set params



(actr-load "ACT-R:tutorial;lisp;zbrodoff.lisp")

(defvar *starting-seed*)

(defun run-it ()
  (when (probe-file (translate-logical-pathname "ACT-R:examples;act-r-solutions;unit-4-zbrodoff-solution-model.lisp"))
    (actr-load "ACT-R:examples;act-r-solutions;unit-4-zbrodoff-solution-model.lisp")
    (setf *starting-seed* 200)
    (zbrodoff-compare 5)))
 

(defun zbrodoff-experiment (&optional (visible (not *run-model*)) (show t))
  (reset)
  (sgp :v nil)
  (suppress-warnings 
   (sgp-fct (list :seed (list (incf *starting-seed*) 0) :lf 0.4 :bll 0.5 :ans 0.5)))
  (setf *trials* nil)
  (dotimes (j 3)
    (dotimes (i 8)
      (setf *trials* (append *trials* (create-set (+ j 1) visible)))))
  (setf *results* nil)
  (collect-responses )
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

