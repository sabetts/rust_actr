;;; Static single-item visual scenario (headless task stub). Everything
;;; device-side happens in task-setup, which the golden driver calls
;;; AFTER reset: reset clears pending visicon additions (and windows do
;;; not survive from load time), so building the window at load never
;;; reaches vision. The model itself is loaded separately by the driver
;;; from attend-test.lisp. No human, no randomness in setup.

(defun task-setup ()
  (let ((win (open-exp-window "attend" :visible nil)))
    (install-device win)
    (add-text-to-exp-window win "Q" :x 100 :y 100)))
