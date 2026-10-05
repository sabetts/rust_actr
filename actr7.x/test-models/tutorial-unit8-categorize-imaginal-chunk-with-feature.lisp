(actr-load "ACT-R:tutorial;lisp;categorize.lisp")

(defun run-it ()
  (mod-buffer-chunk 'imaginal '(eh large))
  (buffer-chunk imaginal))