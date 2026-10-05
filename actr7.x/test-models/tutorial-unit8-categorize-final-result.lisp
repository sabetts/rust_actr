(actr-load "ACT-R:tutorial;lisp;categorize.lisp")


(defun run-it ()
  (mod-buffer-chunk 'imaginal '(category 2 eh large es small nl small mh small))
  (buffer-chunk imaginal))