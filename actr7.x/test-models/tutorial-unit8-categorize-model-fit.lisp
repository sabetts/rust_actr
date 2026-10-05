(actr-load "ACT-R:tutorial;lisp;categorize.lisp")
(actr-load "ACT-R:examples;act-r-solutions;unit-8-categorize-solution-model.lisp")

(defvar *seed-val* 101)

; turn off trace and set seed values and restore default model parameters

(defun trial (stim)
  (reset)
  (sgp :v nil)
  (sgp-fct (list :seed (list (incf *seed-val*) 0)))
  (sgp :mp 1 :ppm 1 :egs .25 :ans .25)
  
  (dolist (x (permute-list (mapcar 'cons *slots* stim)))
    (present-one-attribute-fct (car x) (cdr x)))
  
  (schedule-set-buffer-chunk 'goal '(state categorize) 0)
  (run 20)
  (let ((category (chunk-slot-value-fct (buffer-read 'imaginal) 'category)))
    
    (cond ((not (numberp category))
           (model-output "Model did not provide a category.")
           (cons 0 0))
          ((= 1 category)
           (cons 1 1))
          ((= 2 category)
           (cons 0 1))
          (t 
           (model-output "Model provided invalid category.")
           (cons 0 0)))))

; change the spacing in the output and send to *out-file* directly

(defun categorize-fct (n offset slots)
  
  (if (numberp offset)
      (setf *attribute-offset* offset)
    (setf *attribute-offset* 0))
  
  (cond ((null slots)
         (setf *slots* '(EH ES NL MH)))
        ((not (= (length slots) 4))
         (print-warning "Must specify exactly 4 slot names.  Using defaults instead of ~S" slots)
         (setf *slots* '(EH ES NL MH)))
        ((not (every 'symbolp slots))
         (print-warning "Slot names must be symbols.  Using defaults instead of ~S" slots)
         (setf *slots* '(EH ES NL MH)))
        ((some (lambda (x) (> (count x slots) 1)) slots)
         (print-warning "Slot names must be unique. Using defaults instead of ~S" slots)
         (setf *slots* '(EH ES NL MH)))
        ((find 'category slots)
         (print-warning "Examples already have a category slot. Using defaults instead of ~S" slots)
         (setf *slots* '(EH ES NL MH)))
        (t (setf *slots* slots)))
  
  (let ((results (make-list (length *cat-data*) :initial-element 0))
        (counts (make-list (length *cat-data*) :initial-element 0)))
    (dotimes (i n)
      (let ((data (do-experiment)))
        (setf results (mapcar '+ results (car data)))
        (setf counts (mapcar '+ counts (cdr data)))))
    (setf results (mapcar (lambda (x) (/ x n)) results))
    
    (setf *attribute-offset* 0)
    
    (correlation results *cat-data* *out-file*)
    (mean-deviation results *cat-data* *out-file*)
    
    (format *out-file* "P(C=1)~%")
    (format *out-file* "        ~{(~3d) ~}~%" counts)
    (format *out-file* "data  ~{~6,3f~}~%" *cat-data*)
    (format *out-file* "model ~{~6,3f~}~%" results)))


(defun run-it ()
  (setf *seed-val* 200)
  (categorize-experiment 100))
  