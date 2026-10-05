;;; Similarities for the mp-test golden, applied post-reset (reset
;;; rebuilds chunk structs, wiping table state set at load).

(defun task-setup ()
  (set-similarities (aaa aac -0.02) (aab aac -0.2)))
