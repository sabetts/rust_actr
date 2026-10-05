
(actr-load "ACT-R:test-models;models;unit6-chunk-type-default-slots.lisp")

(defun run-it ()
  (model-output "? (define-chunks~%   (c1 isa a)~%   (c2 isa b)~%   (c3 isa b slot 3)~%   (c4 isa c)~%   (c5 isa c slot2 nil))")
  (model-output "~S" (define-chunks   (c1 isa a)   (c2 isa b)   (c3 isa b slot 3)   (c4 isa c)   (c5 isa c slot2 nil)))
  (model-output "~%? (pprint-chunks c1 c2 c3 c4 c5)")
  (model-output "~S" (pprint-chunks c1 c2 c3 c4 c5))
  )

