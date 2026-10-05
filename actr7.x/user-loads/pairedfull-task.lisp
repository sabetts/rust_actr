;;; Full paired task, two pairs x one block (headless, deterministic):
;;; fixed 5 s prompt/study segments with window clears, single 20 s run.
;;; t=0 probe bank; t=5 study 0; t=10 probe card; t=15 study 1; end 20.
;;; Model file loaded separately by the driver.

(defvar *pairedfull-win* nil)

(defun pf-show (text x y)
  (add-text-to-exp-window *pairedfull-win* text :x x :y y))

(defun pf-clear ()
  (clear-exp-window *pairedfull-win*))

(defun task-setup ()
  (setf *pairedfull-win* (open-exp-window "pairedfull" :visible nil))
  (install-device *pairedfull-win*)
  (pf-show "bank" 100 100)
  (schedule-event 5 'pf-clear :output nil)
  (schedule-event 5 'pf-show :params '("0" 300 100) :output nil)
  (schedule-event 10 'pf-clear :output nil)
  (schedule-event 10 'pf-show :params '("card" 100 200) :output nil)
  (schedule-event 15 'pf-clear :output nil)
  (schedule-event 15 'pf-show :params '("1" 300 200) :output nil)
  (schedule-event 20 'dummy-event-function
                   :maintenance t :priority :min :output nil))
