
(ssp :starting-parameters (:model-warnings nil :testing-function set-testing-traces))

(actr-load "ACT-R:test-models;models;declarative-issues-v9.lisp")


(defun run-it ()
  (sgp :v nil)
  (record-history "retrieval-history")
  (run 27.15)
  (run-until-time 27.381)
  (sgp :v t)
  (dolist (x (no-output (sdm - arg1 nil - arg2 nil)))
    (let ((l (split-string-by-lines (printed-chunk-activation-trace x 27155)))
          (r nil)
          (d nil))
      (dolist (i l)
        (if (or (search "Computing activation" i :test 'string=)
                (search "spreading" i :test 'string-equal)
                (search "sources" i :test 'string-equal)
                (search "transient" i :test 'string-equal)
                (search "has an activation" i :test 'string=))
            (progn
              (setf d nil)
              (model-output i))
          (unless d
            (setf d t)
            (model-output "...")))))))


(defun split-string-by-lines (s)
  (let ((returns (find #\return s))
        (newlines (find #\newline s))
        (marker nil))
    (cond ((and returns newlines)
           (setf marker #\newline)
           (setf s (remove #\return s)))
          (returns
           (setf marker #\return))
          (newlines
           (setf marker #\newline))
          (t
           (return-from split-string-by-lines (list s))))
    (do* ((c s (subseq c (1+ p)))
          (p (position marker c) (position marker c))
          (res (list (subseq c 0 p)) (push (subseq c 0 p) res)))
        ((null p) (reverse res)))))
