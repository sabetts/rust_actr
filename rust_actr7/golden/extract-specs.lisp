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
  '("command-output" "with-open-file" "read-line" "multiple-value-bind"
    "add-pre-event-hook" "add-post-event-hook")
  "Stub text that means the ref is *not replayable* from the spec, so the
stub is classified without executing it. Two kinds qualify:

  - source scrapes (`with-open-file`/`read-line`/`multiple-value-bind`):
    the stub re-reads the tutorial source and echoes a fragment, so the ref
    validates the tutorial text, not the engine;
  - Lisp event hooks (`add-pre/post-event-hook`): the hook body is arbitrary
    Lisp (`evt-time`/`format-event`/`get-event-by-id`), which the engine
    cannot execute — the documented \"no arbitrary Lisp\" divergence.

Everything else is *executed* and its commands recorded (see
`*generic-commands*`); a command the engine cannot reproduce now fails
visibly against the ref instead of being silently skipped.
`(sgp :v …)`/`(sgp-fct (list :v …))`/`*out-file*` are only trace toggles and
targets, not command output, so they must not classify a stub (they made
`tutorial-unit5modeling-v7-p4-run` etc. look like command specs).
`model-output` is handled (`w-evaluate-act-r-command` records a literal
echo), and a filtered `model-output` fragment is caught by the ref check.")

(defparameter *device-tokens*
  '("install-device" "open-exp-window" "add-text-to-exp-window"
    "add-line-to-exp-window" "add-audio-to-exp-window" "modify-text"
    "remove-text" "add-attachment-to-exp-window")
  "Stub text that means the test drives a device: out of scope for now.")

(defparameter *driver-tokens*
  '()
  "Run-it operations the engine has no equivalent for. Production
disable/enable and `pbreak`/`sdp`/`spp`/`whynot` are now recorded commands
(`*generic-commands*`), so this list is empty; it remains as a hook for
operations that turn out not to be replayable.")

(defparameter *generic-commands*
  '(("new-tone-sound" . new-tone-sound)
    ("pp" . pp-fct)
    ("whynot" . whynot-fct)
    ("whynot-dm" . whynot-dm-fct)
    ("dm" . dm-fct)
    ("pprint-chunks" . pprint-chunks-fct)
    ("all-productions" . all-productions)
    ("buffers" . buffers)
    ("buffer-status" . buffer-status)
    ("mp-queue-text" . mp-queue-text)
    ("used-production-buffers" . used-production-buffers)
    ("print-conflict-tree" . print-conflict-tree)
    ("print-visicon" . print-visicon)
    ("printed-visicon" . printed-visicon)
    ("print-dm-finsts" . print-dm-finsts)
    ("print-activation-trace" . print-activation-trace)
    ("print-chunk-activation-trace" . print-chunk-activation-trace-fct)
    ("mod-chunk" . mod-chunk-fct)
    ("merge-dm" . merge-dm-chunks-fct)
    ("chunk-type" . chunk-type-fct)
    ("extend-possible-slots" . extend-possible-slots)
    ("mod-buffer-chunk" . mod-buffer-chunk-fct)
    ("overwrite-buffer-chunk" . overwrite-buffer-chunk-fct)
    ("schedule-simple-mod-buffer-chunk" . schedule-simple-mod-buffer-chunk)
    ("schedule-simple-set-buffer-chunk" . schedule-simple-set-buffer-chunk)
    ("sji" . sji-fct)
    ("sdp" . sdp-fct)
    ;; Task-level state changes a stub makes outside a run. These are the
    ;; calls a *task function* (not the stub's own top level) makes to drive a
    ;; trial, so missing them left the replayed model in the wrong state — the
    ;; same class of gap as `mod-focus-fct` (GOTCHAS #234). Each is recorded as
    ;; `(:command NAME ARGS)` and replayed through the engine's command table.
    ("add-dm" . add-dm-fct)
    ("set-buffer-chunk" . set-buffer-chunk)
    ("clear-buffer" . clear-buffer)
    ("schedule-event-now" . schedule-event-now)
    ("schedule-mod-buffer-chunk" . schedule-mod-buffer-chunk)
    ("schedule-set-buffer-chunk" . schedule-set-buffer-chunk)
    ("add-visicon-features" . add-visicon-features)
    ("delete-visicon-features" . delete-visicon-features)
    ("modify-visicon-features" . modify-visicon-features)
    ("permute-list" . permute-list)
    ("correlation" . correlation)
    ("mean-deviation" . mean-deviation))
  "Commands to record generically, as `(:command NAME ARG…)`, mapped to the
function the stubs' macros expand to. Each is recorded only when it is not
already covered by a dedicated step (`install-wrappers`); the re-entrancy
guard `*recording-command*` suppresses a command a recorded command calls
internally (`no-output` → its argument command).")

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
;; Re-entrancy guard for the generic command wrappers: while one recorded
;; command runs, a command it calls internally (`no-output "spp" …`) is not
;; recorded again.
(defvar *recording-command* nil)
;; `*out-file*` is a real stream (the model-tester binds it to the ref file),
;; so a stub that does `(let ((*error-output* *out-file*)) …)` or `(format
;; *out-file* …)` works. It is a null stream: the extractor records steps, not
;; output, and `w-sgp` maps the stream back to `T` when a stub says
;; `(sgp :v *out-file*)`.
(defvar *null-stream* (make-broadcast-stream))
(defvar *out-file* *null-stream*)
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
(defvar *device-installs* nil)
;; Buttons and lines the task draws (`add-button-to-exp-window` /
;; `add-line-to-exp-window`), as `(run ms …)`. Both become visicon features, and
;; the model's initial visual-location is chosen from them, so a device task
;; cannot be replayed without them (the bst `build-display`). The button's
;; `:action` is recorded verbatim as a form string so a later button-press path
;; can dispatch it.
(defvar *device-buttons* nil)
(defvar *device-lines* nil)
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
  ;; A dedicated wrapper owns this call, so a command its underlying function
  ;; invokes internally (`buffer-chunk` → `pprint-chunks`) must not also be
  ;; recorded: bind the generic guard for the duration.
  (let ((*recording-command* t))
    (apply (gethash name *orig-fns*) args)))

(defvar *last-sgp-params* nil)

(defun normalize-params (params)
  "Replace a stream value (`(sgp :v *out-file*)`) with `T` so the recorded
step is printable and matches what the engine expects."
  (mapcar (lambda (x) (if (streamp x) t x)) params))

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

;; `ssp :starting-parameters (:seed (S C))` is applied by the model-tester's
;; `central-parameters-reset` hook on every `(reset)` (see
;; `central-parameters.lisp:140`), so the effective run seeds depend on it.
;; The single-threaded load does not apply it, so capture and re-apply it
;; ourselves — otherwise the recorded run seeds (and the RNG stream) diverge
;; from the reference (paired-learning).
(defvar *starting-seed* nil)

(defun find-starting-seed (params)
  (let ((sp (loop for (k v) on params when (eq k :starting-parameters) return v)))
    (when (consp sp)
      (loop for (k v) on sp when (eq k :seed) return v))))

(defun find-testing-function (tree)
  (cond ((and (consp tree) (eq (car tree) :testing-function)) (cadr tree))
        ((consp tree)
         (or (find-testing-function (car tree))
             (find-testing-function (cdr tree))))
        (t nil)))

(defun flush-sgp-params ()
  ;; The `:testing-function` re-applies `:act`/`:epl`/`:pct`/`:alpha` on each
  ;; reset (`paired`'s `set-paired-params`, `set-testing-traces`); it is called
  ;; from `w-reset` (the model-tester applies it on reset). Record a step when
  ;; those change so the engine turns tracing/compilation/learning on and off
  ;; per run.
  (when (and (eq *mode* :drive) (not *suppress*))
    (let* ((act (eff-param :act)) (epl (eff-param :epl)) (pct (eff-param :pct))
           ;; The testing function may also change `:alpha` (paired-learning
           ;; runs its second task with `:alpha 1.0`); record it so utility
           ;; learning matches.
           (alpha (eff-param :alpha))
           (cur (list (and act t) (and epl t) (and pct t) alpha)))
      (unless (equal cur *last-sgp-params*)
        (push (list :sgp :act (first cur) :epl (second cur) :pct (third cur)
                    :alpha (fourth cur))
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
    ;; The run's trace flag is the model's `:v` state (the harness's
    ;; `set-testing-traces` sets it to `t` at reset; a stub may hide a segment
    ;; with `(sgp :v nil)`, e.g. `tutorial-unit5modeling-v7-p4-run`).
    (flush-sgp-params)
    (flush-rng-count)
    (let ((rec (list run-time (eff-seed) (eff-v)
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
    (let ((rec (list run-time (eff-seed) (eff-v)
                     (ignore-errors (max-module-name-length)))))
      (push rec *runs*)
      (push (cons :run-full-time rec) *steps*)))
  (let ((*in-run* t))
    (call-orig 'run-full-time run-time nil)))

(defun w-evaluate-act-r-command (name &rest parameters)
  ;; A literal `(model-output "? (run 10)")` echo (the tutorial stubs echo the
  ;; driver call before running): the macro routes through
  ;; `evaluate-act-r-command "model-output"` (`misc-utils.lisp:801`), so record
  ;; it as a step. A format directive or args means the text is computed and
  ;; cannot be replayed from the spec.
  (when (and (eq *mode* :drive) (not *suppress*) (not *in-run*)
             (stringp name) (string-equal name "model-output")
             (stringp (first parameters))
             (not (find #\~ (first parameters))))
    (push (list :model-output (first parameters)) *steps*))
  (apply (gethash 'evaluate-act-r-command *orig-fns*) name parameters))

(defun w-run-until-time (end-time &optional real-time)
  ;; `run-until-time` (`scheduling.lisp:1030`): run to an *absolute* model
  ;; time (used to trace the tail of a long run, e.g.
  ;; `tutorial-unit5modeling-v7-p4-run`). Recorded as its own step.
  (declare (ignore real-time))
  (when (and (eq *mode* :drive) (not *suppress*))
    (flush-sgp-params)
    (flush-rng-count)
    (let ((rec (list end-time (eff-seed) (eff-v)
                     (ignore-errors (max-module-name-length)))))
      (push rec *runs*)
      (push (cons :run-until-time rec) *steps*)))
  (let ((*in-run* t))
    (call-orig 'run-until-time end-time nil)))

(defun w-run-n-events (event-count &optional real-time)
  ;; `run-n-events` (`scheduling.lisp:1081`): run until `event-count` events
  ;; have been processed (used to skip a trace prefix, e.g.
  ;; `tutorial-unit1-semantic-g2-trace`).
  (declare (ignore real-time))
  (when (and (eq *mode* :drive) (not *suppress*))
    (flush-sgp-params)
    (flush-rng-count)
    (let ((rec (list event-count (eff-seed) (eff-v)
                     (ignore-errors (max-module-name-length)))))
      (push rec *runs*)
      (push (cons :run-n-events rec) *steps*)))
  (let ((*in-run* t))
    (call-orig 'run-n-events event-count nil)))

(defun w-goal-focus (spec)
  ;; Only record driver-level goal-focus calls; calls made *during* a run
  ;; (e.g. from a scheduled task command) are reproduced by the engine's
  ;; command dispatch, not replayed as steps.
  (when (and (eq *mode* :drive) (not *suppress*) (not *in-run*))
    (incf *goal-focus-count*)
    (push (list :goal-focus spec) *steps*))
  (call-orig 'goal-focus-fct spec))

(defun w-mod-focus (spec)
  ;; `(mod-focus-fct (slot value ...))` (`goal.lisp:378`): modify the goal
  ;; buffer's chunk. Tasks use it to drive a trial — e.g. `fan-sentence` sets
  ;; the goal's `arg1`/`arg2` (`tutorial/lisp/fan-no-pm.lisp:57`), without which
  ;; the model's first production cannot match and the run ends at t=0. Recorded
  ;; as a flat `(:mod-focus SLOT VALUE ...)` step.
  (when (and (eq *mode* :drive) (not *suppress*) (not *in-run*))
    (push (cons :mod-focus (copy-list spec)) *steps*))
  (call-orig 'mod-focus-fct spec))

(defun w-reset ()
  (when (and (eq *mode* :drive) (not *suppress*))
    (incf *reset-count*)
    (push (list :reset) *steps*))
  (let ((*suppress* t))
    (call-orig 'reset)
    ;; Emulate `central-parameters-reset`: re-apply the stub's
    ;; `:starting-parameters` seed after every reset.
    (when *starting-seed*
      (call-orig 'sgp-fct (list :seed *starting-seed*)))
    ;; ...and its `:testing-function` (`set-testing-traces` sets `:v t`, which
    ;; is why a model with `:v nil` still produces a trace in the harness).
    (when *testing-function*
      (ignore-errors (funcall *testing-function*)))))

(defun w-sgp (params)
  (when (and (eq *mode* :drive) (not *suppress*)
             ;; Drop read-only getters like `(sgp :alpha)` (GOTCHAS #12).
             (not (every #'keywordp params)))
    (let ((rec (normalize-params params)))
      (push rec *sgp-calls*)
      (push (cons :sgp rec) *steps*)))
  (call-orig 'sgp-fct params))

(defun w-ssp (params)
  (let ((tf (find-testing-function params)))
    (when tf (setf *testing-function* tf)))
  (let ((ss (find-starting-seed params)))
    (when ss (setf *starting-seed* ss)))
  (when (and (eq *mode* :drive) (not *suppress*)
             (not (every #'keywordp params)))
    (let ((rec (normalize-params params)))
      (push (cons :ssp rec) *sgp-calls*)
      (push (cons :ssp rec) *steps*)))
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

(defun w-load-code (&rest args)
  ;; `load-act-r-code` (a task file: defuns, experiment code) — recorded like
  ;; `load-act-r-model` but dispatched to its own original.
  (when (eq *mode* :load)
    (let ((file (first args)))
      (when (stringp file) (push file *models*))))
  (apply (gethash 'load-act-r-code *orig-fns*) args))

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

;; `(add-button-to-exp-window win :text "A" :x 5 :y 23 :width 40 :height 24
;;                            :action (...) :color "gray")`: the box *and* its
;; label are visicon features. Record what the engine needs to rebuild them.
(defun w-add-button (win &rest keys)
  (let ((key (apply (gethash 'add-button-to-exp-window *orig-fns*) win keys)))
    (when (and (eq *mode* :drive) (not *suppress*))
      (push (list *reset-count* (mp-time-ms)
                  (or (getf keys :text) "")
                  (or (getf keys :x) 0) (or (getf keys :y) 0)
                  (or (getf keys :width) 0) (or (getf keys :height) 0)
                  (string-downcase (string (or (getf keys :color) "gray")))
                  (unless (null (getf keys :action))
                    (format nil "~s" (getf keys :action))))
            *device-buttons*))
    key))

;; `(add-line-to-exp-window win (list x1 y1) (list x2 y2) 'black)`.
(defun w-add-line (win start end &optional (color 'black))
  (let ((key (call-orig 'add-line-to-exp-window win start end color)))
    (when (and (eq *mode* :drive) (not *suppress*))
      (push (list *reset-count* (mp-time-ms)
                  (first start) (second start) (first end) (second end)
                  (string-downcase (string color)))
            *device-lines*))
    key))

;; `(install-device window)`: only a flag is recorded. The engine needs it
;; because the speech module notifies an installed device instead of
;; scheduling a traced `default-speech-action` (`speech.lisp:267`), so a
;; bare spec (which cannot replay the device) must still know one exists.
(defun w-install-device (device &rest keys)
  (when (and (eq *mode* :drive) (not *suppress*))
    (push (list *reset-count*) *device-installs*))
  (apply (gethash 'install-device *orig-fns*) device keys))

;;; A scheduled command event (`(schedule-event-relative t "cmd" :params ...)`)
;;; prints `NONE <action> <params>` at its `:output` level. Record it so the
;;; engine can replay the line (the ref is the oracle).

(defun record-command (absolute? time action keys)
  ;; Only *task-setup* commands (scheduled outside a run) need replaying: a
  ;; command scheduled during a run comes from a command the engine already
  ;; dispatches (`ul-issues`'s `present-choose`/`show-result` chain) or an
  ;; internal module event it already emits (`audio`'s `new-sound`), so a
  ;; sidecar entry would duplicate the line.
  ;; A recorded command's *internal* schedule must not add a sidecar line:
  ;; `new-sound-event` is reached from `new-tone-sound`, whose call is itself
  ;; replayed and makes the engine emit `new-sound` from the sound object
  ;; (`audio.lisp:733`), so recording the line too would duplicate it.
  (when (and (stringp action) (not *suppress*) (not *in-run*)
             (not *recording-command*))
    (let ((output (getf keys :output t)))
      (when output
        (let* ((ms (if (getf keys :time-in-ms)
                       time
                     (round (* time 1000))))
               (ms (if absolute? ms (+ (mp-time-ms) ms)))
               (params (getf keys :params))
               ;; The `:module` is the trace's module column
               ;; (`schedule-event` defaults it to `:none`); `new-sound`
               ;; carries `:audio` (`audio.lisp:738`).
               (module (string-upcase (symbol-name (or (getf keys :module) :none)))))
          (push (list *reset-count* ms (string-downcase (string output)) module
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

;;; ---------------------------------------------------------------- generic
;;;
;;; Commands without a dedicated step are recorded from the function their
;;; stub macro expands to (`*generic-commands*`). A stub calls the command
;;; macro, which expands to `NAME-FCT`; wrapping that symbol catches the
;;; driver-level call with the arguments the RPC layer would receive.

(defun install-generic-command (name sym)
  ;; Skip a symbol a dedicated wrapper already owns (e.g. `buffer-chunk-fct`)
  ;; so its richer step shape wins and it is not recorded twice.
  (when (and (fboundp sym) (not (gethash sym *orig-fns*)))
    (let ((orig (symbol-function sym))
          (cname (intern (string-upcase name))))
      (setf (gethash sym *orig-fns*) orig)
      (setf (symbol-function sym)
            (lambda (&rest args)
              (let ((outer *recording-command*))
                (let ((*recording-command* t))
                  (when (and (eq *mode* :drive) (not *suppress*) (not *in-run*)
                             (not outer))
                    (push (cons :command (cons cname args)) *steps*))
                  (apply orig args)))))
      (pushnew sym *wrapped*))))

(defun install-generic-commands ()
  (dolist (entry *generic-commands*)
    (install-generic-command (first entry) (cdr entry))))

(defun write-device (name)
  (when (or *device-texts* *device-clears* *device-commands* *device-installs*
            *device-buttons* *device-lines*)
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
               (installs (dedupe-preserve (reverse *device-installs*)))
               (buttons (dedupe-preserve (reverse *device-buttons*)))
               (lines (dedupe-preserve (reverse *device-lines*)))
               (runs (sort (remove-duplicates
                            (append (mapcar #'first texts)
                                    (mapcar #'first clears)
                                    (mapcar #'first commands)
                                    (mapcar #'first modifies)
                                    (mapcar #'first installs)
                                    (mapcar #'first buttons)
                                    (mapcar #'first lines)))
                           #'<)))
          (dolist (r runs)
            (format f "(RUN ~a~%" r)
            (dolist (item texts)
              (when (= (first item) r)
                (format f " (ADD-TEXT ~a ~s ~a ~a)~%" (second item) (third item)
                        (fourth item) (fifth item))))
            (dolist (b buttons)
              (when (= (first b) r)
                (format f " (ADD-BUTTON ~a ~s ~a ~a ~a ~a ~s~@[ ~s~])~%"
                        (second b) (third b) (fourth b) (fifth b)
                        (sixth b) (seventh b) (eighth b) (ninth b))))
            (dolist (l lines)
              (when (= (first l) r)
                (format f " (ADD-LINE ~a ~a ~a ~a ~a ~s)~%"
                        (second l) (third l) (fourth l) (fifth l)
                        (sixth l) (seventh l))))
            (dolist (c clears)
              (when (= (first c) r)
                (format f " (CLEAR ~a)~%" (second c))))
            (dolist (c commands)
              (when (= (first c) r)
                (format f " (COMMAND ~a ~a ~a ~s)~%" (second c) (third c) (fourth c)
                        (fifth c))))
            (dolist (m modifies)
              (when (= (first m) r)
                (format f " (MODIFY-TEXT ~a ~s ~a ~a)~%" (second m) (third m)
                        (fourth m) (fifth m))))
            (dolist (c installs)
              (when (= (first c) r)
                (format f " (INSTALL)~%")))
            (format f ")~%")))))))

(defun install-wrappers ()
  (wrap-fn 'act-r-random #'w-act-r-random)
  (wrap-fn 'sdm-fct #'w-sdm)
  (wrap-fn 'record-history-fct #'w-record-history)
  (wrap-fn 'predict-bold-response #'w-predict-bold)
  (wrap-fn 'run #'w-run)
  (wrap-fn 'run-full-time #'w-run-full-time)
  (wrap-fn 'run-until-time #'w-run-until-time)
  (wrap-fn 'run-n-events #'w-run-n-events)
  (wrap-fn 'goal-focus-fct #'w-goal-focus)
  (wrap-fn 'mod-focus-fct #'w-mod-focus)
  (wrap-fn 'install-device #'w-install-device)
  (wrap-fn 'reset #'w-reset)
  (wrap-fn 'sgp-fct #'w-sgp)
  (wrap-fn 'ssp-fct #'w-ssp)
  (wrap-fn 'actr-load #'w-actr-load)
  (wrap-fn 'load-act-r-model #'w-load-model)
  ;; `load-act-r-code` loads a task file (defuns, experiment code) that the
  ;; stub's `run-it` then calls. It defines no model, but the spec's `:MODELS`
  ;; must list it so the engine can replay the task's `defun`s and scheduled
  ;; commands (e.g. `siegler`, `fan`, `bst-learn`).
  (wrap-fn 'load-act-r-code #'w-load-code)
  (wrap-fn 'buffer-chunk-fct #'w-buffer-chunk)
  (wrap-fn 'add-text-to-exp-window #'w-add-text)
  (wrap-fn 'add-button-to-exp-window #'w-add-button)
  (wrap-fn 'add-line-to-exp-window #'w-add-line)
  (wrap-fn 'modify-text-for-exp-window #'w-modify-text)
  (wrap-fn 'clear-exp-window #'w-clear-exp-window)
  (wrap-fn 'pdisable-fct #'w-pdisable)
  (wrap-fn 'penable-fct #'w-penable)
  (wrap-fn 'schedule-event-relative #'w-schedule-event-relative)
  (wrap-fn 'schedule-event #'w-schedule-event)
  (wrap-fn 'evaluate-act-r-command #'w-evaluate-act-r-command)
  (install-generic-commands))

;;; ---------------------------------------------------------------- extraction

(defun scope-of (src)
  (cond
    ((any-token src *command-tokens*) :command)
    ((any-token src *device-tokens*) :device)
    ((any-token src *driver-tokens*) :driver)))

(defun ref-has-run-end-p (name)
  "True when NAME's ref ends a run (`Stopped`), i.e. it is a *full* trace the
engine can replay. A ref assembled with `model-output` from a pre-event hook
(a filtered fragment, e.g. `tutorial-unit3modeling-v3-clear-imaginal`) lacks
it, so those stay command-scoped. This is the Phase B ref-content triage: the
stub source alone is not a reliable signal."
  (let ((p (merge-pathnames (concatenate 'string name ".ref") *tm-dir*)))
    (and (probe-file p)
         (let ((s (ignore-errors (slurp p))))
           (and s (search "Stopped" s :test #'char-equal) t)))))

(defun fresh-state ()
  (setf *models* nil *runs* nil *steps* nil *goal-focus-count* 0
        *reset-count* 0 *sgp-calls* nil *device-texts* nil *device-clears* nil
        *device-commands* nil *device-modifies* nil *device-installs* nil
        *device-buttons* nil *device-lines* nil
        *item-texts* (make-hash-table :test 'equal)
        *item-pos* (make-hash-table :test 'equal)
        *rng-count* 0 *last-sgp-params* nil
        ;; The model-tester sets a global default
        ;; `ssp :starting-parameters (:testing-function set-testing-traces)`
        ;; before loading the stub (`model-tester.cl:356`); a stub may
        ;; override it. Applied at reset by `w-reset`.
        *testing-function* 'set-testing-traces
        *starting-seed* nil))

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
  "Canonicalize the seed inside each run step, mirroring `canon-runs`; other
step shapes pass through. `:run-full-time` is included: its recorded seed is
re-applied by the engine, but a seed that does not match a literal `:seed`
belongs to a time-seeded (RNG-independent) run, so any value is equivalent
and `(100 0)` keeps the spec stable across regens."
  (mapcar (lambda (st)
            (if (and (consp st)
                     (member (car st)
                             '(:run :run-full-time :run-until-time :run-n-events)))
                (let ((r (cdr st)))
                  (list (car st) (first r)
                        (if (seed-reproducible-p (second r) values) (second r) '(100 0))
                        (third r) (fourth r)))
                st))
          steps))

(defun make-spec (name scope &key flags error)
  (let* ((models (dedupe-preserve (mapcar #'logical->rel (reverse *models*))))
         (seedvals (all-seed-values name models))
         (runs (canon-runs (reverse *runs*) seedvals))
         (steps (reverse *steps*))
         ;; A stub's `(ssp :starting-parameters (:seed (S C)))` is evaluated at
         ;; load (before any run) and re-applied by the central-parameters
         ;; reset hook, so it *is* the effective seed — but `w-ssp` only records
         ;; `:drive`-mode calls, so it would otherwise be lost and the engine
         ;; would fall back to the `(100 0)` run-seed placeholder. Record it as
         ;; a starting-parameters step (`apply-sgp` stores it and `reset`
         ;; re-applies it, `central-parameters.lisp:131`).
         (steps (if *starting-seed*
                    (cons (list :sgp :starting-parameters (list :seed *starting-seed*))
                          steps)
                  steps)))
    (list :name name
          :ref (concatenate 'string name ".ref")
          :models models
          :seed-set (if seedvals t nil)
          :runs runs
          :steps (canon-steps steps seedvals)
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
  (fresh-state)
  (let ((lisp (merge-pathnames (concatenate 'string name ".lisp") *tm-dir*))
        (src (ignore-errors (slurp (merge-pathnames (concatenate 'string name ".lisp") *tm-dir*)))))
    (let ((static-scope (scope-of (or src ""))))
      ;; Command stubs need the command layer (out of scope, classified from
      ;; source without executing). Device and driver stubs ARE executed so
      ;; their run steps and device script are captured (the ref is the
      ;; oracle; the spec replays it). A stub with no command token whose ref
      ;; is only a filtered fragment (no run end) is likewise command output.
      (if (or (eq static-scope :command)
              (and (null static-scope) (not (ref-has-run-end-p name))))
          (make-spec name :command :flags (list :command))
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

;; The model-tester harness defines `set-testing-traces` (`(sgp :v t :cmdt t)`,
;; `model-tester.cl:494`); the single-threaded load does not, so provide the
;; real body. Applied at reset by `w-reset`, it re-enables the trace for a
;; model that set `:v nil` (e.g. the production-compilation-issues models).
(unless (fboundp 'set-testing-traces)
  (defun set-testing-traces (&optional arg)
    (declare (ignore arg))
    (sgp-fct (list :v t :cmdt t))))

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
