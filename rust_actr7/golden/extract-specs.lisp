;;;; extract-specs.lisp — Phase 1 of the ACT-R test-models integration.
;;;;
;;;; Runs the ACT-R regression stubs (`actr7.x/test-models/*.lisp`) under a
;;;; thin instrumentation layer and emits one machine-readable `.spec` per
;;;; trace-reference test into `golden/test-models/specs/`. The Rust harness
;;;; (`tests/test_models.rs`) consumes the specs + the checked-in `.ref`
;;;; files; `cargo test` never invokes SBCL.
;;;;
;;;; Why instrument instead of parsing `run-it`:
;;;;   - seeds are sometimes computed at run time, e.g.
;;;;     `(sgp-fct (list :seed (list (incf *starting-seed*) 0)))`;
;;;;   - `goal-focus`/`sgp`/`ssp` are macros (GOTCHAS #7) that expand to
;;;;     `*-fct` functions, so wrapping the functions sees the real calls;
;;;;   - the model-chain loaded by a stub isn't always literal in the stub.
;;;;
;;;; Invoked by `regen-specs.sh` as:
;;;;   sbcl --non-interactive --disable-debugger \
;;;;     --load <actr7.x>/load-single-threaded-act-r.lisp \
;;;;     --load extract-specs.lisp --eval '(spec-main)'
;;;; Env: SPEC_TMDIR (test-models dir), SPEC_OUTDIR, SPEC_LIST (newline file
;;;; of test names to consider).
;;;;
;;;; Only trace-reference tests are considered (those whose `.ref` has
;;;; timestamped lines). Tests whose stub text mentions a command-layer,
;;;; device, or driver op are classified and written as out-of-scope
;;;; *without* executing `run-it` (avoids slow real experiments and
;;;; time-random seeds); everything else is executed so its run steps and
;;;; effective seeds can be recorded.

(defparameter *command-tokens*
  '("model-output" "command-output" "(sdp" "(spp" "(whynot" "(dm " "(dm)"
    "buffer-chunk" "buffer-status" "(sji" "(pprint" "(sgp-fct (list :v"
    "(sgp :v" "print-conflict-tree" "print-visicon" "print-all-visicon"
    "*out-file*" "with-open-file" "read-line" "multiple-value-bind")
  "Stub text that means the ref contains non-trace output: out of scope.")

(defparameter *device-tokens*
  '("install-device" "open-exp-window" "add-text-to-exp-window"
    "add-line-to-exp-window" "add-audio-to-exp-window" "modify-text"
    "remove-text" "add-attachment-to-exp-window")
  "Stub text that means the test drives a device: out of scope for now.")

(defparameter *driver-tokens*
  '("(pdisable" "(penable" "(pbreak" "(punbreak" "(sdp" "(spp" "(whynot")
  "Run-it operations the engine has no equivalent for (production
disable/enable, breakpoints); not trace-only drivers.")

;;; ---------------------------------------------------------------- state

(defvar *tm-dir* nil)
(defvar *out-dir* nil)
(defvar *actr-dir* nil)
(defvar *orig-fns* (make-hash-table))
(defvar *mode* nil)                 ; :load | :drive | nil
(defvar *models* nil)
(defvar *runs* nil)
(defvar *steps* nil)
(defvar *goal-focus-count* 0)
(defvar *reset-count* 0)
(defvar *in-run* nil)
;; Draws from `act-r-random` made by the task outside a run (setup),
;; recorded as a `(:consume-rng n)` step before the next run so the engine's
;; stream aligns with ACT-R's.
(defvar *rng-count* 0)
(defvar *sgp-calls* nil)
(defvar *suppress* nil)
(defvar *wrapped* nil)
;; Device calls made by an executed stub (`add-text-to-exp-window` etc.),
;; as `(ms text x y)` / `ms`; written to `<name>.device.sexp`.
(defvar *device-texts* nil)
(defvar *device-clears* nil)
;; Scheduled command events that print a trace line (`schedule-event-relative`
;; / `schedule-event` with a string action), as `(ms output details)`.
(defvar *device-commands* nil)
;; Window-item moves (`modify-text-for-exp-window`), as
;; `(run ms text x y)`. The driver's `move-it` is scheduled by the model's
;; `!eval!`, so the engine cannot replay the item position from the model
;; alone; the captured op re-applies it at the same model time.
(defvar *device-modifies* nil)
;; `item-key -> text` and `item-key -> (x . y)` so a modify can record the
;; item's text and the coordinates it does not pass.
(defvar *item-texts* nil)
(defvar *item-pos* nil)

;;; ---------------------------------------------------------------- utils

(defun slurp (path)
  (with-open-file (s path :direction :input :element-type 'character)
    (let ((out (make-string-output-stream)))
      (loop for line = (read-line s nil :eof)
            until (eq line :eof)
            do (write-string line out) (terpri out))
      (get-output-stream-string out))))

(defun any-token (src tokens)
  (loop for tok in tokens thereis (search tok src :test #'char-equal)))

(defun logical->rel (path)
  "ACT-R:tutorial;unit1;count.lisp -> tutorial;unit1;count.lisp."
  (let* ((s (if (stringp path) path (namestring path)))
         (i (search "ACT-R:" s :test #'char-equal)))
    (if i (subseq s (+ i 6)) s)))

;;; Explicit-seed detection.
;;;
;;; A run's seed is reproducible only if it matches a literal value in a real
;;; `(sgp ... :seed V ...)` form in the stub or a loaded model file. Parsing
;;; forms with `read` skips comments exactly like Lisp. A per-spec boolean is
;;; not enough: `simple-ppm-test` runs twice before its first `(sgp :seed
;;; (100 1))`, so those runs are time-seeded while the last is fixed.
;;; `(ssp :starting-parameters (:seed ...))` is also deliberately not
;;; trusted: ACT-R rejects that list once `:testing-function` is in it.

(defun collect-seed-values (form acc)
  (when (consp form)
    ;; Scan this list's elements for a `:seed` followed by its value, so both
    ;; `(sgp :seed V ...)` and `(sgp-fct (list :seed V ...))` are seen.
    (loop for (k v) on form
          when (eq k :seed) do (push v acc))
    (dolist (e form)
      (when (consp e)
        (setf acc (collect-seed-values e acc)))))
  acc)

(defun file-seed-values (path)
  (when (probe-file path)
    (with-open-file (f path :direction :input)
      (let ((acc nil))
        (loop for form = (handler-case (read f nil :eof) (error () :eof))
              until (eq form :eof)
              do (setf acc (collect-seed-values form acc)))
        acc))))

(defun model-phys (logical)
  (merge-pathnames (substitute #\/ #\; (logical->rel logical)) *actr-dir*))

(defun all-seed-values (name logical-models)
  (let ((vals (file-seed-values (merge-pathnames (concatenate 'string name ".lisp") *tm-dir*))))
    (dolist (m logical-models vals)
      (setf vals (append (file-seed-values (model-phys m)) vals)))))

(defun seed-reproducible-p (seed values)
  (and (member seed values :test #'equal) t))

;;; ---------------------------------------------------------------- wrappers

(defun wrap-fn (name wrapper)
  (unless (gethash name *orig-fns*)
    (setf (gethash name *orig-fns*) (symbol-function name)))
  (setf (symbol-function name) wrapper)
  (pushnew name *wrapped*))

(defun call-orig (name &rest args)
  (apply (gethash name *orig-fns*) args))

(defvar *last-sgp-params* nil)

(defun eff-param (key)
  "Read a parameter without recording the call."
  (ignore-errors
    (let ((*suppress* t))
      (let ((r (sgp-fct (list key))))
        ;; `sgp-fct` with a getter returns the value(s): a single value as a
        ;; one-element list (`(T)`), a plist as `((:key v))`.
        (cond ((and (consp r) (consp (car r))) (second (car r)))
              ((consp r) (car r))
              (t r))))))

(defvar *testing-function* nil)

(defun find-testing-function (tree)
  (cond ((and (consp tree) (eq (car tree) :testing-function)) (cadr tree))
        ((consp tree)
         (or (find-testing-function (car tree))
             (find-testing-function (cdr tree))))
        (t nil)))

(defun flush-sgp-params ()
  ;; The `:testing-function` re-applies `:act`/`:epl`/`:pct` on each reset
  ;; (`paired`'s `set-paired-params`); record a step when they change so the
  ;; engine turns tracing/compilation on and off per run.
  (when (and (eq *mode* :drive) (not *suppress*))
    ;; The stub's `ssp :starting-parameters (:testing-function fn)` is applied
    ;; by the model-tester on each reset; the single-threaded load does not,
    ;; so call it here (it sets `:act`/`:epl`/`:pct` from the task's globals).
    (when *testing-function*
      ;; Suppress the harness trace setup (`set-testing-traces` sets
      ;; `:v`/`:cmdt`, already encoded in each `:RUN` step).
      (let ((*suppress* t))
        (ignore-errors (funcall *testing-function*))))
    (let* ((act (eff-param :act)) (epl (eff-param :epl)) (pct (eff-param :pct))
           (cur (list (and act t) (and epl t) (and pct t))))
      (unless (equal cur *last-sgp-params*)
        (push (list :sgp :act (first cur) :epl (second cur) :pct (third cur))
              *steps*)
        (setf *last-sgp-params* cur)))))

(defun eff-seed ()
  (ignore-errors
    (let ((*suppress* t))
      (let ((r (sgp-fct (list :seed))))
        (cond ((and (consp r) (consp (car r))) (list (first (car r)) (second (car r))))
              ((consp r) r)
              (t nil))))))

(defun eff-v ()
  (ignore-errors
    (let ((*suppress* t))
      (let ((r (sgp-fct (list :v))))
        (if (consp r) (and (first r) t) (and r t))))))

(defun w-record-history (&rest names)
  ;; `record-history` (`history-recorder.lisp:614`): the driver names the
  ;; buffers the BOLD predictor traces (`"buffer-trace"` is the recorder
  ;; itself). Record them so the engine can run the demand recorder.
  (when (and (eq *mode* :drive) (not *suppress*) (not *in-run*))
    (let ((buffers (remove "buffer-trace" names :test #'string-equal)))
      (when buffers
        (push (cons :record-history
                    (mapcar (lambda (n) (intern (string-upcase n))) buffers))
              *steps*))))
  (apply (gethash 'record-history-fct *orig-fns*) names))

(defun w-predict-bold (&rest args)
  ;; `predict-bold-response` (`other-files/bold.lisp:648`): print the BOLD
  ;; demand table from the recorded buffer trace.
  (when (and (eq *mode* :drive) (not *suppress*) (not *in-run*))
    (push (list :predict-bold) *steps*))
  (apply (gethash 'predict-bold-response *orig-fns*) args))

(defun w-sdm (spec)
  ;; `sdm` (`commands/dm-commands.lisp:484`): the stub calls
  ;; `(sdm isa pair)` between tasks to print the matching DM chunks; record
  ;; the spec so the engine can reproduce the dump.
  (when (and (eq *mode* :drive) (not *suppress*) (not *in-run*))
    (push (list* :sdm spec) *steps*))
  (call-orig 'sdm-fct spec))

(defun w-act-r-random (limit)
  ;; `act-r-random` (`random.lisp:482`): the task's own draws (e.g.
  ;; `permute-list` in `paired-task`) happen outside a run; count them so a
  ;; `(:consume-rng n)` step can advance the engine's stream to match.
  (when (and (eq *mode* :drive) (not *suppress*) (not *in-run*))
    (incf *rng-count*))
  (call-orig 'act-r-random limit))

(defun flush-rng-count ()
  (when (and (eq *mode* :drive) (not *suppress*) (> *rng-count* 0))
    (push (list :consume-rng *rng-count*) *steps*))
  (setf *rng-count* 0))

(defun w-run (run-time &optional real-time)
  ;; Force non-real-time: pacing is wall-clock only and would make regen slow
  ;; without changing model output.
  (declare (ignore real-time))
  (when (and (eq *mode* :drive) (not *suppress*))
    ;; The trace-reference harness monitors the model-trace regardless of the
    ;; model's `:v` (which only gates the stdout echo), so the recorded run is
    ;; always traced.
    (flush-sgp-params)
    (flush-rng-count)
    (let ((rec (list run-time (eff-seed) t
                     (ignore-errors (max-module-name-length)))))
      (push rec *runs*)
      (push (cons :run rec) *steps*)))
  (let ((*in-run* t))
    (call-orig 'run run-time nil)))

(defun w-run-full-time (run-time &optional real-time)
  ;; `run-full-time` (`scheduling.lisp:986`): run for `run-time` seconds of
  ;; model time, guaranteed by a hidden dummy event — recorded as its own
  ;; step so the engine runs to the limit instead of stopping early.
  (declare (ignore real-time))
  (when (and (eq *mode* :drive) (not *suppress*))
    (flush-sgp-params)
    (flush-rng-count)
    (let ((rec (list run-time (eff-seed) t
                     (ignore-errors (max-module-name-length)))))
      (push rec *runs*)
      (push (cons :run-full-time rec) *steps*)))
  (let ((*in-run* t))
    (call-orig 'run-full-time run-time nil)))

(defun w-goal-focus (spec)
  ;; Only record driver-level goal-focus calls; calls made *during* a run
  ;; (e.g. from a scheduled task command) are reproduced by the engine's
  ;; command dispatch, not replayed as steps.
  (when (and (eq *mode* :drive) (not *suppress*) (not *in-run*))
    (incf *goal-focus-count*)
    (push (list :goal-focus spec) *steps*))
  (call-orig 'goal-focus-fct spec))

(defun w-reset ()
  (when (and (eq *mode* :drive) (not *suppress*))
    (incf *reset-count*)
    (push (list :reset) *steps*))
  (let ((*suppress* t))
    (call-orig 'reset)))

(defun w-sgp (params)
  (when (and (eq *mode* :drive) (not *suppress*)
             ;; Drop read-only getters like `(sgp :alpha)` (GOTCHAS #12).
             (not (every #'keywordp params)))
    (push params *sgp-calls*)
    (push (cons :sgp params) *steps*))
  (call-orig 'sgp-fct params))

(defun w-ssp (params)
  (let ((tf (find-testing-function params)))
    (when tf (setf *testing-function* tf)))
  (when (and (eq *mode* :drive) (not *suppress*)
             (not (every #'keywordp params)))
    (push (cons :ssp params) *sgp-calls*)
    (push (cons :ssp params) *steps*))
  (call-orig 'ssp-fct params))

(defun w-actr-load (&rest args)
  (when (eq *mode* :load)
    (let ((file (first args)))
      (when (stringp file) (push file *models*))))
  (apply (gethash 'actr-load *orig-fns*) args))

(defun w-load-model (&rest args)
  (when (eq *mode* :load)
    (let ((file (first args)))
      (when (stringp file) (push file *models*))))
  (apply (gethash 'load-act-r-model *orig-fns*) args))

(defun w-buffer-chunk (names)
  ;; Task drivers often end with `(buffer-chunk ...)`; record it so the
  ;; spec can reproduce the command-layer dump (z-threads).
  (when (and (eq *mode* :drive) (not *suppress*) (not *in-run*))
    (push (cons :buffer-chunk names) *steps*))
  (call-orig 'buffer-chunk-fct names))

;;; ---------------------------------------------------------------- device
;;;
;;; Device stubs are now executed (not classified out of scope from source)
;;; so their run steps and window script are captured. The script is emitted
;;; as `<name>.device.sexp` in the format the Rust harness consumes; a
;;; hand-authored sidecar (which carries task knobs the extractor cannot
;;; derive, e.g. CONSUME-RNG / CLEAR-ON-KEYPRESS) is never overwritten.

(defun w-add-text (win text &rest keys)
  (let ((key (apply (gethash 'add-text-to-exp-window *orig-fns*) win text keys)))
    (when (and (eq *mode* :drive) (not *suppress*))
      (push (list *reset-count* (mp-time-ms) text (or (getf keys :x) 0) (or (getf keys :y) 0))
            *device-texts*)
      (when key
        (setf (gethash key *item-texts*) text)
        (setf (gethash key *item-pos*)
              (cons (or (getf keys :x) 0) (or (getf keys :y) 0)))))
    key))

(defun w-modify-text (item &rest keys)
  (when (and (eq *mode* :drive) (not *suppress*))
    (let* ((txt (gethash item *item-texts*))
           (cur (gethash item *item-pos*))
           (x (or (getf keys :x) (car cur) 0))
           (y (or (getf keys :y) (cdr cur) 0)))
      (when txt
        (push (list *reset-count* (mp-time-ms) txt x y) *device-modifies*)
        (setf (gethash item *item-pos*) (cons x y)))))
  (apply (gethash 'modify-text-for-exp-window *orig-fns*) item keys))

(defun w-clear-exp-window (&optional win)
  (when (and (eq *mode* :drive) (not *suppress*))
    (push (list *reset-count* (mp-time-ms)) *device-clears*))
  (call-orig 'clear-exp-window win))

;;; A scheduled command event (`(schedule-event-relative t "cmd" :params ...)`)
;;; prints `NONE <action> <params>` at its `:output` level. Record it so the
;;; engine can replay the line (the ref is the oracle).

(defun record-command (absolute? time action keys)
  (when (and (stringp action) (not *suppress*))
    (let ((output (getf keys :output t)))
      (when output
        (let* ((ms (if (getf keys :time-in-ms)
                       time
                     (round (* time 1000))))
               (ms (if absolute? ms (+ (mp-time-ms) ms)))
               (params (getf keys :params)))
          (push (list *reset-count* ms (string-downcase (string output))
                      (format nil "~a~{ ~a~}" action params))
                *device-commands*))))))

(defun w-schedule-event-relative (time action &rest keys)
  (when (eq *mode* :drive)
    (record-command nil time action keys))
  (apply (gethash 'schedule-event-relative *orig-fns*) time action keys))

(defun w-schedule-event (time action &rest keys)
  (when (eq *mode* :drive)
    (record-command t time action keys))
  (apply (gethash 'schedule-event *orig-fns*) time action keys))

(defun w-pdisable (productions)
  (when (and (eq *mode* :drive) (not *suppress*))
    (push (cons :pdisable productions) *steps*))
  (call-orig 'pdisable-fct productions))

(defun w-penable (productions)
  (when (and (eq *mode* :drive) (not *suppress*))
    (push (cons :penable productions) *steps*))
  (call-orig 'penable-fct productions))

(defun write-device (name)
  (when (or *device-texts* *device-clears* *device-commands*)
    (let ((path (merge-pathnames (concatenate 'string name ".device.sexp") *out-dir*)))
      (when (probe-file path)
        (format t "SPEC-NOTE ~a : keeping hand-authored device sidecar~%" name)
        (return-from write-device nil))
      (with-open-file (f path :direction :output :if-exists :supersede
                              :if-does-not-exist :create)
        ;; Group ops by the reset they belong to (`(RUN n …)`); the task
        ;; re-applies its window content per run, so ops at the same ms in
        ;; different runs must not collide.
        (let* ((texts (dedupe-preserve (reverse *device-texts*)))
               (clears (dedupe-preserve (reverse *device-clears*)))
               (commands (dedupe-preserve (reverse *device-commands*)))
               (modifies (dedupe-preserve (reverse *device-modifies*)))
               (runs (sort (remove-duplicates
                            (append (mapcar #'first texts)
                                    (mapcar #'first clears)
                                    (mapcar #'first commands)
                                    (mapcar #'first modifies)))
                           #'<)))
          (dolist (r runs)
            (format f "(RUN ~a~%" r)
            (dolist (item texts)
              (when (= (first item) r)
                (format f " (ADD-TEXT ~a ~s ~a ~a)~%" (second item) (third item)
                        (fourth item) (fifth item))))
            (dolist (c clears)
              (when (= (first c) r)
                (format f " (CLEAR ~a)~%" (second c))))
            (dolist (c commands)
              (when (= (first c) r)
                (format f " (COMMAND ~a ~a ~s)~%" (second c) (third c) (fourth c))))
            (dolist (m modifies)
              (when (= (first m) r)
                (format f " (MODIFY-TEXT ~a ~s ~a ~a)~%" (second m) (third m)
                        (fourth m) (fifth m))))
            (format f ")~%")))))))

(defun install-wrappers ()
  (wrap-fn 'act-r-random #'w-act-r-random)
  (wrap-fn 'sdm-fct #'w-sdm)
  (wrap-fn 'record-history-fct #'w-record-history)
  (wrap-fn 'predict-bold-response #'w-predict-bold)
  (wrap-fn 'run #'w-run)
  (wrap-fn 'run-full-time #'w-run-full-time)
  (wrap-fn 'goal-focus-fct #'w-goal-focus)
  (wrap-fn 'reset #'w-reset)
  (wrap-fn 'sgp-fct #'w-sgp)
  (wrap-fn 'ssp-fct #'w-ssp)
  (wrap-fn 'actr-load #'w-actr-load)
  (wrap-fn 'load-act-r-model #'w-load-model)
  (wrap-fn 'buffer-chunk-fct #'w-buffer-chunk)
  (wrap-fn 'add-text-to-exp-window #'w-add-text)
  (wrap-fn 'modify-text-for-exp-window #'w-modify-text)
  (wrap-fn 'clear-exp-window #'w-clear-exp-window)
  (wrap-fn 'pdisable-fct #'w-pdisable)
  (wrap-fn 'penable-fct #'w-penable)
  (wrap-fn 'schedule-event-relative #'w-schedule-event-relative)
  (wrap-fn 'schedule-event #'w-schedule-event))

;;; ---------------------------------------------------------------- extraction

(defun scope-of (src)
  (cond
    ((any-token src *command-tokens*) :command)
    ((any-token src *device-tokens*) :device)
    ((any-token src *driver-tokens*) :driver)))

(defun fresh-state ()
  (setf *models* nil *runs* nil *steps* nil *goal-focus-count* 0
        *reset-count* 0 *sgp-calls* nil *device-texts* nil *device-clears* nil
        *device-commands* nil *device-modifies* nil
        *item-texts* (make-hash-table :test 'equal)
        *item-pos* (make-hash-table :test 'equal)
        *rng-count* 0 *last-sgp-params* nil *testing-function* nil))

(defun dedupe-preserve (list)
  (let ((seen (make-hash-table :test 'equal)) (out nil))
    (dolist (x list (nreverse out))
      (unless (gethash x seen)
        (setf (gethash x seen) t)
        (push x out)))))

(defun canon-runs (runs values)
  "Replace a time-random seed with a stable canonical one. A run whose seed
does not match a literal `:seed` value in the sources is RNG-independent (a
noisy run without a fixed seed has a non-reproducible ref and must not be a
golden), so the value is irrelevant."
  (mapcar (lambda (r)
            (list (first r)
                  (if (seed-reproducible-p (second r) values) (second r) '(100 0))
                  (third r) (fourth r)))
          runs))

(defun canon-steps (steps values)
  "Canonicalize the seed inside each `(:run ...)` step, mirroring
`canon-runs`; other step shapes pass through."
  (mapcar (lambda (st)
            (if (and (consp st) (eq (car st) :run))
                (let ((r (cdr st)))
                  (list :run (first r)
                        (if (seed-reproducible-p (second r) values) (second r) '(100 0))
                        (third r) (fourth r)))
                st))
          steps))

(defun make-spec (name scope &key flags error)
  (let* ((models (dedupe-preserve (mapcar #'logical->rel (reverse *models*))))
         (seedvals (all-seed-values name models))
         (runs (canon-runs (reverse *runs*) seedvals)))
    (list :name name
          :ref (concatenate 'string name ".ref")
          :models models
          :seed-set (if seedvals t nil)
          :runs runs
          :steps (canon-steps (reverse *steps*) seedvals)
          :goal-focus *goal-focus-count*
          :resets *reset-count*
          :sgp (reverse *sgp-calls*)
          :scope scope
          :flags (remove nil flags)
          :error error)))

(defun extract-one-attempt (name lisp static-scope)
  (fresh-state)
  (clear-all)
  (let ((*mode* :load))
    (load lisp))

  (let ((*mode* :drive))
    (force-virtual-window (funcall 'run-it)))
  (let ((spec (make-spec
               name
               (cond ((> (length *runs*) 1) :multi)
                     ((> *goal-focus-count* 0) :multi)
                     ((> *reset-count* 0) :multi)
                     ((= (length *runs*) 1) :pure)
                     (t :other))
               :flags (cond ((eq static-scope :device) '(:device))
                            ((eq static-scope :driver) '(:driver))))))
    (write-device name)
    spec))

(defun tutorial-companions ()
  "Load `ACT-R:tutorial;lisp;<stem>.lisp` for each loaded
`tutorial/.../<stem>-model.lisp`. Some stubs' `run-it` calls the tutorial
task function without loading it (`sperling-trial`); this is only tried
after an undefined-function error. Returns whether anything was loaded."
  (let ((any nil))
    (dolist (m (reverse *models*))
      (let* ((rel (logical->rel m))
             (semi (position #\; rel :from-end t))
             (file (if semi (subseq rel (1+ semi)) rel)))
        (when (and file (search "-model.lisp" file :from-end t))
          (let* ((stem (subseq file 0 (- (length file) (length "-model.lisp"))))
                 (cand (merge-pathnames
                        (concatenate 'string "tutorial/lisp/" stem ".lisp")
                        *actr-dir*)))
            (when (probe-file cand)
              (handler-case (progn (load cand) (setf any t))
                (error () nil)))))))
    any))

(defun extract-one (name)
  "Return a spec plist for NAME, or a scope-error spec on failure."
  (let ((lisp (merge-pathnames (concatenate 'string name ".lisp") *tm-dir*))
        (src (ignore-errors (slurp (merge-pathnames (concatenate 'string name ".lisp") *tm-dir*)))))
    (let ((static-scope (scope-of (or src ""))))
      ;; Command stubs need the command layer (out of scope, classified from
      ;; source without executing). Device and driver stubs ARE executed so
      ;; their run steps and device script are captured (the ref is the
      ;; oracle; the spec replays it).
      (if (eq static-scope :command)
          (make-spec name static-scope :flags (list static-scope))
          (handler-case (extract-one-attempt name lisp static-scope)
            (error (e)
              ;; Retry once with the tutorial task file if `run-it` needed an
              ;; undefined function the stub did not load.
              (if (and (search "undefined" (string-downcase (princ-to-string e)))
                       (tutorial-companions))
                  (handler-case (extract-one-attempt name lisp static-scope)
                    (error (e2) (make-spec name :error :error (princ-to-string e2))))
                (make-spec name :error :error (princ-to-string e)))))))))

(defun write-spec (spec)
  (let ((path (merge-pathnames (concatenate 'string (getf spec :name) ".spec") *out-dir*)))
    (with-open-file (f path :direction :output :if-exists :supersede
                            :if-does-not-exist :create)
      (let ((*print-pretty* nil) (*print-circle* nil) (*print-length* nil)
            (*print-level* nil) (*print-case* :upcase))
        (prin1 spec f)
        (terpri f)))))

; The model-tester harness defines `set-testing-traces`; the spec records the
;; trace itself, so a no-op keeps a `:testing-function` (e.g. paired's
;; `set-paired-params`) from erroring before it applies `:act`/`:epl`/`:pct`.
(unless (fboundp 'set-testing-traces)
  (defun set-testing-traces (&optional arg) (declare (ignore arg)) t))

(defun spec-main ()
  (setf *tm-dir* (pathname (concatenate 'string (sb-ext:posix-getenv "SPEC_TMDIR") "/"))
        *out-dir* (pathname (concatenate 'string (sb-ext:posix-getenv "SPEC_OUTDIR") "/"))
        *actr-dir* (pathname (concatenate 'string (sb-ext:posix-getenv "SPEC_ACTRDIR") "/")))
  (ensure-directories-exist *out-dir*)
  ;; Keep the extraction log readable: ACT-R model trace would otherwise
  ;; print in full for every executed test.
  (ignore-errors (turn-off-act-r-output))
  (install-wrappers)
  (let ((names (with-open-file (f (sb-ext:posix-getenv "SPEC_LIST"))
                 (loop for line = (read-line f nil :eof)
                       until (eq line :eof)
                       when (> (length line) 0) collect line))))
    (let ((ok 0) (fail 0))
      (dolist (name names)
        (handler-case
            (let ((spec (extract-one name)))
              (write-spec spec)
              (format t "SPEC-OK ~a scope=~a runs=~a~%" name (getf spec :scope)
                      (length (getf spec :runs)))
              (incf ok))
          (error (e)
            (format t "SPEC-FAIL ~a : ~a~%" name e)
            (incf fail))))
      (format t "~%SPEC-RESULT ok=~a fail=~a~%" ok fail)
      (sb-ext:exit :code (if (zerop fail) 0 1)))))
