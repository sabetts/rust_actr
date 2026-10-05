
(defvar *demo2-trace-filtering*)
(defvar *demo2-disable-trace*)
(defvar *demo2-trace-started*)

(defun filter (evt)
  (when *demo2-disable-trace*
    (setf *demo2-disable-trace* nil)
    (sgp :v nil))
  (when (and (evt-output evt) 
             (filter-test (get-event-by-id evt)))
    (if (and
         (eq (evt-module evt) :vision)
         
         (or (= (evt-mstime evt) 0)
             (= (evt-mstime evt) 885)
             (= (evt-mstime evt) 970)))
        (progn
          (push-last (format-event evt) *demo2-trace-filtering*)
          (setf *demo2-trace-started* (evt-mstime evt)))
      (when (and *demo2-trace-started*
                 (not (string-equal "..." (car (last *demo2-trace-filtering*))))
                 (= (evt-mstime evt) *demo2-trace-started*))
        (push-last "..." *demo2-trace-filtering*)
        (setf *demo2-trace-started* -1))))
    nil)

(defun run-it ()
  (setf *demo2-trace-filtering* nil)
  (setf *demo2-disable-trace* t)
  (setf *demo2-trace-started* nil)
  
  (let ((p (add-pre-event-hook 'filter)))
    (ignore-real-time
     (demo2-experiment))
    (delete-event-hook p))
  (sgp :v t)
  
  (model-output "~{~a~^~%~}" (butlast *demo2-trace-filtering*)))
  
  
  

(actr-load "ACT-R:tutorial;lisp;demo2.lisp")