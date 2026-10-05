
(defun run-it () 
  ;; The internal chunk-type has extra info (the subtypes and extended slots)
  ;; so just pull it out of the audio module...
  
 (with-open-file (f (translate-logical-pathname "ACT-R:core-modules;audio.lisp") :direction :input)
    (loop
      (let ((form (read-line f nil :done)))
        (when (search "(chunk-type sound " form :test 'string-equal)
          (model-output form)
          (return))))))
          
  
(actr-load "ACT-R:tutorial;unit3;sperling-model.lisp")
