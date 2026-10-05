;;; Test model which uses dynamic real-time updating.
;;; Simple situation where it just schedules a new event
;;; in the "past" during the slack hook to make sure
;;; that the conflict-resolution (which is dynamic)
;;; gets bumped back.


(defvar *once* nil)

(defun dynamic-time-test-dummy ())

(defun simple-slack-test (current-time next-time)
  (if (and *once* (> next-time 200))
      (progn
        (schedule-event .5 'dynamic-time-test-dummy)
        (setf *once* nil))
    (sleep (/ (- next-time current-time) 2000))))

(defun run-it ()
  
  (reset)
  (setf *once* t)
  (mp-real-time-management :mode 'interval :time-function 'get-internal-real-time :units-per-second (/ internal-time-units-per-second 2) :slack-function 'simple-slack-test)
  
  
  (run 1.1 t)
  
  (reset)
  (setf *once* t)
  (mp-real-time-management :mode 'interval :time-function 'get-internal-real-time :units-per-second (/ internal-time-units-per-second 2) :slack-function 'simple-slack-test)
  
  (run 1.1 t))

(load-act-r-model "ACT-R:test-models;models;dynamic-real-time.lisp")
