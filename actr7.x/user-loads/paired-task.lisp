;;; Static first-trial paired-associate (headless, deterministic): probe
;;; shown post-reset via task-setup; study digit scheduled at 6 s.
;;; Model file itself is loaded separately by the driver.

(defvar *paired-win* nil)

(defun show-study-item ()
  (add-text-to-exp-window *paired-win* "0" :x 200 :y 200))

(defun task-setup ()
  (setf *paired-win* (open-exp-window "paired" :visible nil))
  (install-device *paired-win*)
  (add-text-to-exp-window *paired-win* "bank" :x 100 :y 100)
  (schedule-event 6 'show-study-item))
