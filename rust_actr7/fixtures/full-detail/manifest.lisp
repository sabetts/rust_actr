;;;; manifest.lisp — full-detail differential traces.
;;;;
;;;; Same schema as ../manifest.lisp, with an optional 6th element: a plist
;;;; of `sgp` settings applied after reset and before the trace redirect.
;;;; These traces are compared byte-exact by tests/full_detail.rs.
(
 ;; Declarative activation + noise, full activation/utility narration.
 ("noisy-count-full" "user-loads;noisy-count.lisp" (100 0) 10 nil
  (:trace-detail high :act high :pct t))
 ;; Base-level learning with the same full narration.
 ("bll-count-full" "user-loads;bll-count.lisp" (100 0) 10 nil
  (:trace-detail high :act high :pct t))
 ;; Partial matching: similarities decide between two non-exact candidates.
 ("mp-test-full" "user-loads;mp-test.lisp" (100 0) 10 "user-loads;mp-task.lisp"
  (:trace-detail high :act high :pct t))
 ;; Utility learning: reward at 275 ms flips the next choice.
 ("ul-test-full" "user-loads;ul-test.lisp" (100 0) 1 "user-loads;ul-task.lisp"
  (:trace-detail high :act high :pct t))
 ;; Vision: find/attend/read a single window item.
 ("attend-full" "user-loads;attend-test.lisp" (100 0) 10 "user-loads;attend-task.lisp"
  (:trace-detail high :act high))
)
