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
)
