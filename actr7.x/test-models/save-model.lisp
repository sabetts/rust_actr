
(require-extra "save-model")

(defun run-it () 
  (with-open-file (f (translate-logical-pathname "ACT-R:test-models;save-model-test.txt") :direction :output :if-exists :supersede :if-does-not-exist :create)
    (format f "DEMO~%"))
  (force-virtual-window
   (ignore-real-time
    (demo2-experiment)))
  (save-model-file "ACT-R:test-models;save-model-test.txt" 
                   :pre-chunk-type-hook 'pcth
                   :pre-chunk-hook 'pch
                   :pre-production-hook 'pph
                   :end-hook 'eh)
  
  (with-open-file (f (translate-logical-pathname "ACT-R:test-models;save-model-test.txt") 
                     :direction :input)
    (loop (let ((r (read-line f nil :done)))
            (when (eq r :done) 
              (return))
            (model-output r))))
    )

(defun pcth ()
  (command-output ";PRE-CHUNK-TYPE-HOOK"))

(defun pch ()
  (command-output ";PRE-CHUNK-HOOK"))

(defun pph ()
  (command-output ";PRE-PRODUCTION-HOOK"))

(defun eh ()
  (command-output ";END-HOOK"))


(actr-load "ACT-R:tutorial;lisp;demo2.lisp")
