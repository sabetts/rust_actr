;;; Not quite sure why this test model was added.
;;; It seems to be verifying that activation spreads
;;; correctly from a reference to a chunk that was
;;; a buffer copy and has since been merged back into DM.



(defun run-it ()
  (run 10))

(load-act-r-model "ACT-R:test-models;models;sji-merge.lisp")
