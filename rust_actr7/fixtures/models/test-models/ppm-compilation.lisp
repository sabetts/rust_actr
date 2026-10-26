
;;; This test model has simple productions which 
;;; may only partially match the buffer contents
;;; and it is testing how production compilation
;;; deals with the situations.
;;;
;;; Unfortunately the way ppm and compilation work
;;; together are a little "unspecified" at this point
;;; because of issues with mismatches.  What happens
;;; when a variable bound in one production happens
;;; in a slot that was tested with a different constant
;;; in the other?  What about when there is a slot comparison
;;; done in one of the productions between mismatching
;;; slots that are similar - which value is 'bound' and
;;; used (not just an issue with compilation actually
;;; and order of tests within the production dictate
;;; what happens).
;;;
;;; So while the output of this is deterministic and
;;; consistent with the basic compilation concepts it 
;;; may not be creating the "right" productions.



(defun run-it ()
  (run 10))

(load-act-r-model "ACT-R:test-models;models;ppm-compilation.lisp")
