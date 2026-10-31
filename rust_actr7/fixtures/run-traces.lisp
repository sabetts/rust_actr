;;;; run-traces.lisp — extra-trace generator driver.
;;;;
;;;; Loaded *after* single-threaded ACT-R in a headless SBCL:
;;;;   sbcl --non-interactive --disable-debugger \
;;;;     --load <actr7.x>/load-single-threaded-act-r.lisp \
;;;;     --load run-traces.lisp \
;;;;     --eval '(trace-main)'
;;;; With env vars TRACE_MANIFEST (manifest.lisp path) and
;;;; TRACE_OUTDIR (directory receiving *.trace + *.meta).
;;;;
;;;; For each manifest entry (name model-subpath seed run-time):
;;;;   load model -> (sgp :seed seed) -> (reset) ->
;;;;   (sgp :v <name.trace>) -> (run run-time) -> (sgp :v t)
;;;;
;;;; Notes:
;;;; - The :v redirect is ACT-R's own trace sink
;;;;   (actr7.x/framework/printing.lisp). The file is byte-identical
;;;;   to the stdout trace (verified by diff).
;;;; - Model-load warnings go to stdout, never into the trace, because
;;;;   the redirect is installed after load/reset and removed after run.
;;;; - (reset) closes :v/:cmdt file streams, so the redirect must be
;;;;   installed *after* reset. Stale traces are deleted first because
;;;;   :v opens files with :if-exists :append.
;;;; - (sgp :seed ...) before (reset) survives reset and makes output
;;;;   byte-deterministic across processes (verified by diff).
;;;;   Without an explicit seed the RNG is time-seeded
;;;;   (actr7.x/framework/random.lisp) and traces vary run to run.

(defun trace-env (name)
  #+sbcl (sb-ext:posix-getenv name)
  #-sbcl (error "trace driver currently requires SBCL (missing env ~a)" name))

(defun trace-run-one (entry outdir)
  (destructuring-bind (name subpath seed runtime &optional taskpath settings) entry
    (let ((trace-path (concatenate 'string outdir "/" name ".trace"))
          (meta-path (concatenate 'string outdir "/" name ".meta")))
      (when (probe-file trace-path)
        (delete-file trace-path))
      (handler-case
          (progn
            ;; Stale task-setup binding leaked from ACT-R startup
            ;; auto-load (user-loads/) must never fire: only an entry
            ;; that defines it gets it, and it is always removed after.
            (let ((had-task-setup (fboundp 'task-setup)))
              (load-act-r-model (concatenate 'string "ACT-R:" subpath))
              ;; Optional task file (device scaffolding, experiment
              ;; code): loaded plainly, since nested load-act-r-model
              ;; is locked out while a model is loading.
              (when taskpath
                (actr-load (concatenate 'string "ACT-R:" taskpath)))
              ;; NB: sgp is a macro that quotes its args
              ;; (framework/parameters.lisp:202), so computed values
              ;; must go through sgp-fct.
              (sgp-fct (list :seed seed))
              (reset)
              ;; Task entries may define (task-setup) to build device
              ;; state that reset would otherwise wipe (e.g. window items
              ;; whose pending visicon additions reset clears).
              (when (and (fboundp 'task-setup) (not had-task-setup))
                (funcall 'task-setup))
              (when (fboundp 'task-setup)
                (fmakunbound 'task-setup))
              ;; Extra `sgp` settings applied after reset, before the trace
              ;; redirect: the full-detail harness asks for `:act high`,
              ;; `:pct t`, `:cst t`, etc. here.
              (when settings
                (sgp-fct settings)))
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
              (format m "task: ~S~%" (or taskpath "none"))
              (when settings
                (format m "settings: ~S~%" settings))
              (format m "act-r-version: ~S~%" (ssp :act-r-version))
              (format m "lisp: ~a ~a~%"
                      (lisp-implementation-type)
                      (lisp-implementation-version)))
            (format t "TRACE-OK ~a -> ~a~%" name trace-path))
        (error (e)
          (ignore-errors (sgp-fct (list :v t)))
          (when (probe-file trace-path)
            (delete-file trace-path))
          (error e))))))

(defun trace-main ()
  (let ((manifest-path (trace-env "TRACE_MANIFEST"))
        (outdir (trace-env "TRACE_OUTDIR"))
        (failures nil))
    (unless (and manifest-path outdir)
      (error "TRACE_MANIFEST and TRACE_OUTDIR must both be set"))
    (ensure-directories-exist (concatenate 'string outdir "/"))
    (let ((manifest (with-open-file (f manifest-path :direction :input)
                      (read f))))
      (dolist (entry manifest)
        (handler-case (trace-run-one entry outdir)
          (error (e)
            (format t "~%TRACE-FAIL ~a : ~a~%" (first entry) e)
            (push (first entry) failures))))
      (if failures
          (progn
            (format t "~%TRACE-RESULT FAIL ~a~%" (reverse failures))
            #+sbcl (sb-ext:exit :code 1)
            #-sbcl (error "trace failures: ~a" failures))
          (format t "~%TRACE-RESULT OK~%")))))
