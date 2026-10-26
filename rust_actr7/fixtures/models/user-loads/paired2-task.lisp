;;; Two-pairs x two-blocks paired-associate (headless, deterministic):
;;; fixed stimulus schedule, no response contingency — the model runs
;;; continuously while probes/studies appear on schedule. Block 1:
;;; bank@0 study0@6 card@15 study1@21. Block 2: card@30 study1@36
;;; bank@45 study0@51. Run 60 s. Model file loaded separately.

(defvar *paired2-win* nil)

(defun show-item (text x y)
  (add-text-to-exp-window *paired2-win* text :x x :y y))

(defun task-setup ()
  (setf *paired2-win* (open-exp-window "paired2" :visible nil))
  (install-device *paired2-win*)
  (show-item "bank" 100 100)
  (dolist (s '((6 "0" 300 100)
               (15 "card" 100 200)
               (21 "1" 300 200)
               (30 "card" 100 200)
               (36 "1" 300 200)
               (45 "bank" 100 100)
               (51 "0" 300 100)))
    (destructuring-bind (when text x y) s
      (schedule-event when 'show-item :params (list text x y) :output nil))))
