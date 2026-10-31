;;;; run-draws.lisp — capture act-r-random / act-r-noise call sequences.
;;;;
;;;; Loaded after single-threaded ACT-R. For each manifest entry:
;;;;   load model -> seed -> reset -> task-setup -> settings -> clear the log
;;;;   -> log on -> (run runtime) -> log off -> write <name>.draws.sexp.
;;;; The wrapper records both the outer act-r-noise call and the inner
;;;; act-r-random 1.0 it makes, matching the engine's logged draw sequence.

(defvar *draw-log* nil)
(defvar *draw-logging* nil)

(let ((orig-random (symbol-function 'act-r-random))
      (orig-noise (symbol-function 'act-r-noise)))
  (setf (symbol-function 'act-r-random)
        (lambda (limit)
          (let ((v (funcall orig-random limit)))
            (when *draw-logging*
              (push (list :random limit v) *draw-log*))
            v)))
  (setf (symbol-function 'act-r-noise)
        (lambda (s)
          (let ((v (funcall orig-noise s)))
            (when *draw-logging*
              (push (list :noise s v) *draw-log*))
            v))))

(defun draw-env (name)
  #+sbcl (sb-ext:posix-getenv name)
  #-sbcl (error "draw driver requires SBCL (missing env ~a)" name))

(defun draw-run-one (entry outdir)
  (destructuring-bind (name subpath seed runtime &optional taskpath settings) entry
    (let ((draw-path (concatenate 'string outdir "/" name ".draws.sexp"))
          (meta-path (concatenate 'string outdir "/" name ".meta")))
      (when (probe-file draw-path) (delete-file draw-path))
      (let ((had-task-setup (fboundp 'task-setup)))
        (load-act-r-model (concatenate 'string "ACT-R:" subpath))
        (when taskpath
          (actr-load (concatenate 'string "ACT-R:" taskpath)))
        (sgp-fct (list :seed seed))
        (reset)
        (when (and (fboundp 'task-setup) (not had-task-setup))
          (funcall 'task-setup))
        (when (fboundp 'task-setup)
          (fmakunbound 'task-setup))
        (when settings
          (sgp-fct settings)))
      (setf *draw-log* nil
            *draw-logging* t)
      (unwind-protect
           (run runtime)
        (setf *draw-logging* nil))
      (with-open-file (f draw-path :direction :output
                         :if-exists :supersede :if-does-not-exist :create)
        (dolist (d (reverse *draw-log*))
          (format f "~s~%" d)))
      (with-open-file (m meta-path :direction :output
                         :if-exists :supersede :if-does-not-exist :create)
        (format m "name: ~a~%" name)
        (format m "model: ~a~%" subpath)
        (format m "seed: ~S~%" seed)
        (format m "run: ~S~%" runtime)
        (format m "task: ~S~%" (or taskpath "none"))
        (when settings
          (format m "settings: ~S~%" settings))
        (format m "act-r-version: ~S~%" (ssp :act-r-version))
        (format m "lisp: ~a ~a~%"
                (lisp-implementation-type) (lisp-implementation-version)))
      (format t "DRAWS-OK ~a~%" name))))

(defun draws-main ()
  (let ((manifest-path (draw-env "DRAWS_MANIFEST"))
        (outdir (draw-env "DRAWS_OUTDIR")))
    (unless (and manifest-path outdir)
      (error "DRAWS_MANIFEST and DRAWS_OUTDIR must both be set"))
    (ensure-directories-exist (concatenate 'string outdir "/"))
    (let ((manifest (with-open-file (f manifest-path :direction :input)
                      (read f))))
      (dolist (entry manifest)
        (draw-run-one entry outdir))
      (format t "DRAWS-RESULT OK~%"))))
