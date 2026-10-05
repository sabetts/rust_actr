


(defun run-it () 
  (dolist (x '(test1 test2 test3 test4 test5 test6 test7 test8 test9)) (run-test x))
  (clear-all)
  (undefine-module motor-extension)
  (setf *modules* (remove "extended-motor-actions" *modules* :test 'string-equal))
  (suppress-warnings
   (load-act-r-code "ACT-R:support;general-pm.lisp")
   (load-act-r-code "ACT-R:core-modules;motor.lisp")
   (defmethod prepare-features ((mtr-mod motor-module) (self punch))
     )
   (defmethod prepare-features ((mtr-mod motor-module) (self peck))
     )
   (defmethod prepare-features ((mtr-mod motor-module) (self peck-recoil))
     ))
  )

(clear-all)
(require-extra "extended-motor-actions")
(load-act-r-model "ACT-R:extras;extended-motor-actions;test-model.lisp")

