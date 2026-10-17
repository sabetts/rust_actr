;;;; manifest.lisp — project-authored "extra trace" model list.
;;;;
;;;; These are NOT ACT-R regression tests: they are scenarios the upstream
;;;; suite does not cover (activation noise, base-level learning, device/
;;;; window scaffolding). They are our own oracle, compared byte-exact by
;;;; tests/extra_traces.rs. The upstream corpus lives in actr7.x/test-models
;;;; and is consumed by tests/test_models.rs.
;;;;
;;;; Schema: ((name model-subpath seed run-time [task-subpath]) ...)
;;;; - name: string, output files are <name>.trace + <name>.meta
;;;; - model-subpath: appended to the ACT-R logical host, e.g.
;;;;   "tutorial;unit1;count.lisp" loads ACT-R:tutorial;unit1;count.lisp
;;;; - seed: (sgp :seed ...) value, a 2-int list. Mandatory: without it
;;;;   the RNG is time-seeded and traces are not reproducible.
;;;; - run-time: seconds passed to (run ...). Must be long enough for the
;;;;   model to halt ("Stopped because no events left to process").
;;;; - task-subpath: optional setup file (device scaffolding); may define
;;;;   (task-setup), which the driver calls once post-reset.
;;
;; Frozen: this list is not expected to grow. Regenerate with ./regen.sh.
(
 ;; Synthetic (project-authored, in user-loads/): count with :ans 0.5
 ;; exercises activation noise + noisy latencies end to end.
 ("noisy-count" "user-loads;noisy-count.lisp" (100 0) 10)
 ;; Same model, seed (7 0): two successful noisy retrievals at non-50ms
 ;; latencies, then a threshold failure.
 ("noisy-count-7" "user-loads;noisy-count.lisp" (7 0) 10)
 ;; Base-level learning, no noise: deterministic rising activation as
 ;; harvests add references (exercises merge-on-clear + decay equation).
 ("bll-count" "user-loads;bll-count.lisp" (100 0) 10)
 ;; Static single-item visual task (model + task files + device sidecar):
 ;; find, attend, read, press-key.
 ("attend" "user-loads;attend-test.lisp" (100 0) 10 "user-loads;attend-task.lisp")
 ;; Partial matching: neither candidate matches exactly; similarities
 ;; decide (model + task + sidecar similarities).
 ("mp-test" "user-loads;mp-test.lisp" (100 0) 10 "user-loads;mp-task.lisp")
 ;; Penalty-flip utility learning (model + task + sidecar reward).
 ("ul-test" "user-loads;ul-test.lisp" (100 0) 1 "user-loads;ul-task.lisp")
)
