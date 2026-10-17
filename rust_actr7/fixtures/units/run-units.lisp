;;;; run-units.lisp â€” unit-table oracle driver.
;;;;
;;;; Loaded *after* single-threaded ACT-R in a headless SBCL:
;;;;   sbcl --non-interactive --disable-debugger \
;;;;     --load <actr7.x>/load-single-threaded-act-r.lisp \
;;;;     --load run-units.lisp \
;;;;     --eval '(units-main)'
;;;; With env var GOLDEN_UNITS_DIR (directory receiving *.sexp tables).
;;;;
;;;; Evaluates pure ACT-R functions over input grids and prints
;;;; machine-readable rows the Rust tests assert against. The latency
;;;; table needs no model; the base-level table loads one (for params
;;;; and the mp clock) but runs nothing.
;;;;
;;;; Tables:
;;;; - latency.sexp: (LAT A LF LE RESULT), (ROUND X RESULT).
;;;; - base-level.sexp: (NOW T), (ACTSCALE BLL VALUE),
;;;;   (BL BLL OL ACTSCALE N CT REF... RESULT).

(defun units-env (name)
  #+sbcl (sb-ext:posix-getenv name)
  #-sbcl (error "units driver currently requires SBCL (missing env ~a)" name))

(defun units-main ()
  (let ((outdir (units-env "GOLDEN_UNITS_DIR")))
    (unless outdir
      (error "GOLDEN_UNITS_DIR must be set"))
    (ensure-directories-exist (concatenate 'string outdir "/"))
    (with-open-file (f (concatenate 'string outdir "/latency.sexp")
                       :direction :output
                       :if-exists :supersede
                       :if-does-not-exist :create)
      (format f "; latency + round oracle: (LAT A LF LE RESULT) (ROUND X RESULT)~%")
      (format f "; A and LE are single-floats (coerced, as on the engine path); LF is integer ms~%")
      ;; CL round is half-to-even: probe exact halves in single precision.
      (dolist (x '(0.5 1.5 2.5 3.5 12.5 -0.5 -2.5))
        (let ((sx (coerce x 'single-float)))
          (format f "(ROUND ~S ~D)~%" sx (round sx))))
      ;; Activation grid: A in 0..4 step .01, two LE values, LF=50ms.
      (dolist (le '(1.0 0.5))
        (let ((sle (coerce le 'single-float)))
          (dotimes (i 401)
            (let ((a (coerce (* i 0.01) 'single-float)))
              (format f "(LAT ~S 50 ~S ~D)~%"
                      a sle (compute-activation-latency a 50 sle)))))))
    ;; Base-level decay grid. compute-references takes the dm instance
    ;; but never touches it with sact=nil, so nil is passed; act-scale
    ;; comes from sgp. References sit at (now - k*lag), creation earlier.
    (load-act-r-model "ACT-R:tutorial;unit1;count.lisp")
    (flet ((act-scale-now (dm)
             (dm-act-scale dm)))
      (with-open-file (f (concatenate 'string outdir "/base-level.sexp")
                         :direction :output
                         :if-exists :supersede
                         :if-does-not-exist :create)
        (format f "; base-level oracle: (NOW T) (ACTSCALE BLL VALUE) (BL BLL OL ACTSCALE N CT REF... RESULT)~%")
        (format f "; OL is NIL, a number, or T; times are integer ms; floats single~%")
        (with-model-eval 'count
          (let ((dm (get-module-fct 'declarative))
                (now (mp-time-ms)))
            (format f "(NOW ~D)~%" now)
            (dolist (bll '(0.3 0.5))
              (let ((sbll (coerce bll 'single-float)))
                (sgp-fct (list :bll sbll))
                (let ((scale0 (act-scale-now dm)))
                  (let ((*print-readably* t))
                    (format f "(ACTSCALE ~S ~S)~%" sbll scale0)))
                (dolist (ol '(nil 6 t))
                  (dolist (spec '((1 50) (2 500) (3 2000) (8 500)))
                    (destructuring-bind (n lag) spec
                      (let ((refs (loop for k from 1 to n
                                        collect (- now (* k lag))))
                            (ct (- now (* n lag) 1000))
                            (scale (act-scale-now dm)))
                        (let ((*print-readably* t))
                          (format f "(BL ~S ~S ~S ~D ~D~{ ~D~} ~S)~%"
                                  sbll ol scale n ct refs
                                  (compute-references dm n refs ct (- sbll)
                                                      nil ol scale nil))))))))))))
    ;; Spreading SJI grid: fan-model DM has real fan structure.
    ;; J structs come from the DM list (model-table structs lack fan
    ;; values); fan-in keys are name symbols. Absent pairs legitimately
    ;; yield 0.0 and are recorded with FANIN NIL.
    (load-act-r-model "ACT-R:tutorial;unit5;fan-model.lisp")
    (with-model-eval 'fan
      (let* ((dm (get-module-fct 'declarative))
             (chunks (all-dm-chunks dm))
             (find-chunk (lambda (n)
                           (find n chunks :key #'act-r-chunk-name :test #'eq))))
        (with-open-file (f (concatenate 'string outdir "/sji.sexp")
                           :direction :output
                           :if-exists :supersede
                           :if-does-not-exist :create)
          (format f "; sji oracle: (SJI J I MAS NSJI FANOUT CFANOUT INDM FANIN RESULT)~%")
          (format f "; J/I are chunk names; FANIN NIL means J absent from I's fan-in (-> 0.0)~%")
          (dolist (mas '(1.0 1.5))
            (let ((smas (coerce mas 'single-float)))
              (dolist (nsji '(nil t))
                (dolist (pair '((HIPPIE P1) (HIPPIE P4) (PARK P1) (PARK P4)
                                (GIANT P8) (GIANT P11) (CASTLE P9) (CASTLE P11)
                                (BANK P3) (BANK P6) (LAWYER P13) (STORE P13)
                                (HIPPIE P13) (STORE P1) (GIANT P1) (PARK P13)))
                  (destructuring-bind (jn iname) pair
                    (let ((j (funcall find-chunk jn))
                          (i (funcall find-chunk iname)))
                      (if (or (null j) (null i))
                          (format t "; SJI-SKIP ~S ~S not in DM~%" jn iname)
                          (let* ((fi (chunk-fan-in i))
                                 (hit (assoc jn fi))
                                 (fanin (when hit (cdr hit))))
                                 (indm (if (chunk-in-dm j) 'T 'NIL))
                                 (res (let ((*print-readably* t))
                                        (compute-sji j i nil smas nsji))))
                            (let ((*print-readably* t))
                              (format f "(SJI ~S ~S ~S ~S ~D ~D ~S ~S ~S)~%"
                                      jn iname smas (if nsji 'T 'NIL)
                                      (chunk-fan-out j) (chunk-c-fan-out j)
                                      indm (if fanin fanin 'NIL) res))))))))))))
    (format t "~%UNITS-RESULT OK~%"))))
