;;;; manifest.lisp — M0 golden model list.
;;;; Schema: ((name model-subpath seed run-time) ...)
;;;; - name: string, output files are <name>.trace + <name>.meta
;;;; - model-subpath: appended to the ACT-R logical host, e.g.
;;;;   "tutorial;unit1;count.lisp" loads ACT-R:tutorial;unit1;count.lisp
;;;; - seed: (sgp :seed ...) value, a 2-int list. Mandatory: without it
;;;;   the RNG is time-seeded and traces are not reproducible.
;;;; - run-time: seconds passed to (run ...). Must be long enough for the
;;;;   model to halt ("Stopped because no events left to process").
(
 ("count"    "tutorial;unit1;count.lisp"    (100 0) 10)
 ("semantic" "tutorial;unit1;semantic.lisp" (100 0) 10)
 ("addition" "tutorial;unit1;addition.lisp" (100 0) 10)
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
 ;; find, attend, read, press-key. First device golden.
 ("attend" "user-loads;attend-test.lisp" (100 0) 10 "user-loads;attend-task.lisp")
)
