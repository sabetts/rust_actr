;;; This was designed as a test of the ability to use a child type's slots
;;; when using a parent type specification.  With ACT-R 6.1 things are
;;; even less restricted than that since "isa"s are optional now and a
;;; better test should be written.


(defun run-it ()
  (let ((*error-output* *out-file*))
    (matching-test)
    (retrieval-test)
    (imaginal-test)))

(defun matching-test ()
  (matching)
  (dolist (goal '(parent target-nil target-full other)) 
    (set-buffer-chunk 'goal goal) 
    (format *out-file* "+++++++++++++++++++++++++~%") 
    (buffer-chunk goal) 
    (whynot)))

(defun retrieval-test ()
  (retrieving)
  (dotimes (i 9)
    (goal-focus-fct (car (define-chunks-fct `((isa test count ,i)))))
    (pp-fct (list (intern (format nil "RETRIEVE~d" i))))
    (run 10)))

(defun imaginal-test ()
  (imaginal)
  (with-parameters (:trace-detail high)
    (dotimes (i 5)
      (goal-focus-fct (car (define-chunks-fct `((isa test count ,i)))))
      (set-buffer-chunk 'imaginal (car (define-chunks (isa target slot1 slot1))))
      (pp-fct (list (intern (format nil "IMAGINAL~d" i))))
      (run 10))))


(load-act-r-model "ACT-R:test-models;models;child-slots.lisp")
