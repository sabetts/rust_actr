;;; Penalty-flip utility learning (headless, deterministic): reward
;;; -10 at 275 ms punishes whoever fired (L, by tie order), flipping
;;; subsequent selections to R.

(defun task-setup ()
  (schedule-event 0.275 'trigger-reward :params '(-10) :output nil))
