;;; Test model to verify that a merged chunk
;;; will affect production matching even
;;; without a direct change to the chunk in
;;; the buffer.  The goal buffer has a slot 
;;; value set to the chunk in the imaginal buffer
;;; which doesn't match the underlying chunk in
;;; dm (b as tested by p3) but after the chunk
;;; in the imaginal buffer is cleared it merges
;;; with the chunk b and production p4 then
;;; matches even though the goal chunk wasn't
;;; changed.



(defun run-it ()
  (run 1))

(load-act-r-model "ACT-R:test-models;models;merging-test.lisp")
