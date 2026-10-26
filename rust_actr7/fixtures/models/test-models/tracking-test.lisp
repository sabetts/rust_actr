;;; This model does a fairly thorough test of the visual tracking
;;; to make sure that chunks in the vision module buffers are/aren't
;;; overwritten or modified as appropriate.

(defvar *x*)

(defun run-it ()
  
  (reset)
  (let ((window (open-exp-window "Moving X")))
    (setf *letter* (add-text-to-exp-window window "x" :x 10 :y 150))
    (setf *x* 10)
    (install-device window)
    (run 3)))


(defun move-it () 
  (incf *x* 10)
  (modify-text-for-exp-window *letter* :x *x*))



(load-act-r-model "ACT-R:test-models;models;tracking-test.lisp")
