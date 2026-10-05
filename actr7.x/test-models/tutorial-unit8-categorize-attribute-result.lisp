(actr-load "ACT-R:tutorial;lisp;categorize.lisp")


(defun run-it ()
  (mod-buffer-chunk 'imaginal '(size small))
  (buffer-chunk imaginal))