
(defvar *started*)

(defun run-it () 
  (define-model f (sgp :v t))
  (setf *started* nil)
  (with-open-file (f (translate-logical-pathname "ACT-R:test-models;unit-3-subitize-solution.ref") :direction :input)
    (loop
      (let ((form (read-line f nil :done)))
        (when (eq form :done)
          (return))
        (when (or *started*
                  (search "correlation" form :test 'string-equal))
          (setf *started* t)
          (model-output form))))))
                  
  
