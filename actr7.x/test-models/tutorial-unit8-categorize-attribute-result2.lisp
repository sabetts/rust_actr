(actr-load "ACT-R:tutorial;lisp;categorize.lisp")


(defun run-it ()
  (extend-possible-slots 's2)
  (mod-buffer-chunk 'imaginal '(size small s2 large))
  (buffer-chunk imaginal))