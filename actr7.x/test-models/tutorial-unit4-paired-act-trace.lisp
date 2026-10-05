
;; Custom do-experiment function to change model paramers (:act t).



(actr-load "ACT-R:tutorial;lisp;paired.lisp")

(defvar *starting-seed*)
(defvar *disable-trace*)


(defvar *output-started* nil)

(defun filter (evt)
  (when *disable-trace*
    (setf *disable-trace* nil)
    (sgp :act t :v nil)
    )
  (if (eq (evt-action evt) 'start-retrieval)
      (progn
        (sgp-fct (list :v *out-file*))
        (setf *output-started* t)
        )
    (if *output-started*
        (progn
          (model-output "...")
          (sgp :v nil)
          (setf *output-started* nil))
      ))

  nil)

(defun run-it ()
  (setf *output-started* nil)
  (setf *disable-trace* t)
  
  (let ((p (add-pre-event-hook 'filter)))
    (ignore-real-time
     (paired-task 1 2))
    (delete-event-hook p)))
   

