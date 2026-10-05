

(defun run-it () 
  (force-virtual-window
   (ignore-real-time
    (demo2-emma)))
  (clear-all)
  (undefine-module :emma)
  (setf *modules* (remove "emma" *modules* :test 'string-equal))
  (remove-act-r-command-monitor "installing-vision-device" "emma-detect-device-install")
  (remove-act-r-command-monitor "visual-clear" "emma-clear-action"))

(actr-load "ACT-R:extras;emma;example-demo2.lisp")
