

(defun run-it ()
  (dolist (mod '(t nil))
    (dolist (over '(t nil))
      (dolist (unstuff '(nil t .25 .35))
        (run-test unstuff over mod)))))

(defun run-test (unstuff over mod)
  (reset)
  (when mod
    (goal-focus free))
  (sgp-fct (list :unstuff-visual-location unstuff :overstuff-visual-location over))
  (install-device (open-exp-window "" :visible nil))
  (add-text-to-exp-window nil "a" :x 50 :y 100)
  (model-output "unstuff = ~s over = ~s modify ~:[no~;yes~]" unstuff over mod)
  (run-until-time .3)
  (add-text-to-exp-window nil "b" :x 10 :y 100)
  (run-until-time 1)
  (add-text-to-exp-window nil "c" :x 5 :y 100)
  (run-until-time 1.05)
  (add-text-to-exp-window nil "d" :x 0 :y 100)
  (run-until-time 2))
    

(load-act-r-model "ACT-R:test-models;models;unstuff-visual.lisp")
     
     
