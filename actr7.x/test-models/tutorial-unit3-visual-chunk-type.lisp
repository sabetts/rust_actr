
(defun run-it () 
  ;; The internal chunk-type has extra info (the subtypes and extended slots)
  ;; so just pull it out of the audio module...
  
 (with-open-file (f (translate-logical-pathname "ACT-R:core-modules;vision.lisp") :direction :input)
    (loop
      (let ((form (read-line f nil :done)))
        (when (search "(chunk-type visual-object" form :test 'string-equal)
          (model-output form)
          (return))))))
          
  
(actr-load "ACT-R:tutorial;unit3;sperling-model.lisp")
