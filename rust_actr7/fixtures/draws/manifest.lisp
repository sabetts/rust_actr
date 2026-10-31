;;;; manifest.lisp — draw-sequence differential cases.
;;;;
;;;; Schema matches ../manifest.lisp:
;;;;   (name model-subpath seed run-time [task-subpath [settings]])
;;;; The generator logs every act-r-random/act-r-noise call made by (run ...)
;;;; after reset/settings; tests/draws.rs replays the same model and compares.
(
 ("noisy-count-draws" "user-loads;noisy-count.lisp" (100 0) 10 nil
  (:trace-detail high :act high :pct t))
 ("noisy-count-7-draws" "user-loads;noisy-count.lisp" (7 0) 10 nil
  (:trace-detail high :act high :pct t))
)
