;;; Test some of the :nearest options for visual-location requests.


(defvar *which*)

(defun which-one ()
  *which*)

(defun run-it ()
  (add-act-r-command "which-one" 'which-one)
  
  (dolist (x '(current current-x current-y clockwise counterclockwise))
    
    (reset)
    ;; use a new window each time since open-exp-window
    ;; sets the center automatically (install-device does not)
    
    (install-device (open-exp-window "" :visible nil :width 200 :height 120))
    
    (setf *which* x)
    
    (clear-exp-window)
    (add-text-to-exp-window nil "x" :x 30 :y 40)
    (add-text-to-exp-window nil "y" :x 40 :y 30)
    (add-text-to-exp-window nil "z" :x 32 :y 32)
    (run-n-events 2)
    (print-visicon)
    
    ;; explicitly attend to 0,0 at the start
    ;; instead of letting it be the default and
    ;; printing warnings.
    
    (module-request 'visual (define-chunk-spec cmd move-attention screen-pos start))
    
    (run 10))
  (remove-act-r-command "which-one"))

(load-act-r-model "ACT-R:test-models;models;nearest-vis-loc.lisp")
