;;; Performance test of the basic declarative retrieval matching.
;;; 
;;; - One target chunk, and a variable number of mismatching ones.
;;;
;;; - All learning turned off.
;;;
;;; - A fixed number of retrieval requests will be made - 10000. 
;;;
;;; - Runs the model making the requests as it goes.
;;;
;;; - Sizes will be the number of other chunks of the same type
;;;   in DM.

(setf *performance-test* '(retrieval-set-size-s-init 
                           retrieval-set-size-s-run
                           (1 10 100 250 500 750 1000)))

(defvar *count* nil)
(defvar *c2* nil)

(defvar *last-set* nil)
(defvar *last-result* nil)

(defun save-last-set (x)
  (setf *last-set* x)
  nil)

(defun save-last-result (x)
  (setf *last-result* x)
  nil)

(defun retrieval-set-size-s-init (n)
  (define-model test (sgp :esc t :v nil :ncnar nil :bll .5 :mas 3 :nsji nil :ol 2 :retrieval-set-hook save-last-set :retrieved-chunk-hook save-last-result)
    (chunk-type mem-test slot1 slot2 slot3) 
    (add-dm (a isa chunk) (g isa mem-test slot1 g slot2 a slot3 t)))
  (dotimes (i n)
    (let ((name (new-name "mem-test")))
      (add-dm-fct `((,name isa mem-test slot1 ,(if (<= i (floor n 3)) 'g name) slot2 a)))
      (set-base-levels-fct `((,name 10 -1000)))))
  (add-post-event-hook 'count-and-request)
  (setf *count* 0)
  (setf *c2* 0)
  (goal-focus g))
  

(defun count-and-request (event)
  (when (and (eq (evt-module event) 'declarative)
             (eq (evt-action event) 'set-buffer-chunk))
    (incf *count*)
    (when (eq (second (evt-params event)) 'g)
      (incf *c2*))
    (schedule-module-request 'retrieval (define-chunk-spec isa mem-test slot1 g) .05)))


(defun retrieval-set-size-s-run (n)
  (declare (ignore n))
  (schedule-module-request 'retrieval (define-chunk-spec isa mem-test slot1 g) .05)
  (run-until-condition (lambda (x) (declare (ignore x)) (= *count* 10000)))
  (unless (= *c2* 10000) (break)))
