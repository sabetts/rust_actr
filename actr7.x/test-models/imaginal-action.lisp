;;; Model tests the imaginal-action buffer requests.

(defparameter *counter* 0)

(defun make-simple-chunk ()
  (car (define-chunks (isa simple count 0))))

(defun no-chunk-generated ()
  nil)

(defun update-imaginal (slot)
  (schedule-mod-buffer-chunk 'imaginal (list slot (incf *counter*)) .150 :module 'imaginal)
  (schedule-event-relative .150 'set-imaginal-free :module 'imaginal :priority -10))


(defun run-it ()
  (add-act-r-command "update-imaginal" 'update-imaginal)
  (add-act-r-command "no-chunk-generated" 'no-chunk-generated)
  (add-act-r-command "make-simple-chunk" 'make-simple-chunk)
  (reset)
  (setf *counter* 0)
  (run 5)
  (remove-act-r-command "update-imaginal")
  (remove-act-r-command "no-chunk-generated")
  (remove-act-r-command "make-simple-chunk"))


(load-act-r-model "ACT-R:test-models;models;imaginal-action.lisp")
