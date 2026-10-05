;;;; run-golden.lisp — M0 golden-trace generator driver.
;;;;
;;;; Loaded *after* single-threaded ACT-R in a headless SBCL:
;;;;   sbcl --non-interactive --disable-debugger \
;;;;     --load <actr7.x>/load-single-threaded-act-r.lisp \
;;;;     --load run-golden.lisp \
;;;;     --eval '(golden-main)'
;;;; With env vars GOLDEN_MANIFEST (manifest.lisp path) and
;;;; GOLDEN_OUTDIR (directory receiving *.trace + *.meta).
;;;;
;;;; For each manifest entry (name model-subpath seed run-time):
;;;;   load model -> (sgp :seed seed) -> (reset) ->
;;;;   (sgp :v <name.trace>) -> (run run-time) -> (sgp :v t)
;;;;
;;;; Notes:
;;;; - The :v redirect is ACT-R's own trace sink
;;;;   (actr7.x/framework/printing.lisp). The file is byte-identical
;;;;   to the stdout trace (verified by diff in M0).
;;;; - Model-load warnings go to stdout, never into the trace, because
;;;;   the redirect is installed after load/reset and removed after run.
;;;; - (reset) closes :v/:cmdt file streams, so the redirect must be
;;;;   installed *after* reset. Stale traces are deleted first because
;;;;   :v opens files with :if-exists :append.
;;;; - (sgp :seed ...) before (reset) survives reset and makes output
;;;;   byte-deterministic across processes (verified by diff in M0).
;;;;   Without an explicit seed the RNG is time-seeded
;;;;   (actr7.x/framework/random.lisp) and traces vary run to run.

(defun golden-env (name)
  #+sbcl (sb-ext:posix-getenv name)
  #-sbcl (error "golden driver currently requires SBCL (missing env ~a)" name))

(defun golden-run-one (entry outdir)
  (destructuring-bind (name subpath seed runtime) entry
    (let ((trace-path (concatenate 'string outdir "/" name ".trace"))
          (meta-path (concatenate 'string outdir "/" name ".meta")))
      (when (probe-file trace-path)
        (delete-file trace-path))
      (handler-case
          (progn
            (load-act-r-model (concatenate 'string "ACT-R:" subpath))
            ;; NB: sgp is a macro that quotes its args
            ;; (framework/parameters.lisp:202), so computed values
            ;; must go through sgp-fct.
            (sgp-fct (list :seed seed))
            (reset)
            (sgp-fct (list :v trace-path))
            (unwind-protect
                 (run runtime)
              (sgp-fct (list :v t)))
            (with-open-file (m meta-path
                               :direction :output
                               :if-exists :supersede
                               :if-does-not-exist :create)
              (format m "name: ~a~%" name)
              (format m "model: ~a~%" subpath)
              (format m "seed: ~S~%" seed)
              (format m "run: ~S~%" runtime)
              (format m "act-r-version: ~S~%" (ssp :act-r-version))
              (format m "lisp: ~a ~a~%"
                      (lisp-implementation-type)
                      (lisp-implementation-version)))
            (format t "GOLDEN-OK ~a -> ~a~%" name trace-path))
        (error (e)
          (ignore-errors (sgp-fct (list :v t)))
          (when (probe-file trace-path)
            (delete-file trace-path))
          (error e))))))

(defun golden-main ()
  (let ((manifest-path (golden-env "GOLDEN_MANIFEST"))
        (outdir (golden-env "GOLDEN_OUTDIR"))
        (failures nil))
    (unless (and manifest-path outdir)
      (error "GOLDEN_MANIFEST and GOLDEN_OUTDIR must both be set"))
    (ensure-directories-exist (concatenate 'string outdir "/"))
    (let ((manifest (with-open-file (f manifest-path :direction :input)
                      (read f))))
      (dolist (entry manifest)
        (handler-case (golden-run-one entry outdir)
          (error (e)
            (format t "~%GOLDEN-FAIL ~a : ~a~%" (first entry) e)
            (push (first entry) failures))))
      (if failures
          (progn
            (format t "~%GOLDEN-RESULT FAIL ~a~%" (reverse failures))
            #+sbcl (sb-ext:exit :code 1)
            #-sbcl (error "golden failures: ~a" failures))
          (format t "~%GOLDEN-RESULT OK~%")))))
