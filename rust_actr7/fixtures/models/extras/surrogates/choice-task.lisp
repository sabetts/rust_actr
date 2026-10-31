;; Project-authored choice surrogate task: 12 alternating rewards.
(defparameter *feedback* '(-1 1 -1 1 -1 1 -1 1 -1 1 -1 1))
(defun run-it ()
  (dotimes (i (length *feedback*))
    (goal-focus-fct '(isa choose state choose))
    (run 2)
    (trigger-reward (nth i *feedback*))))
