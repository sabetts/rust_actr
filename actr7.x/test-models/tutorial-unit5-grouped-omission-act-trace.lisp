(defvar *set-break*)

(defun filter (evt)
  (when *set-break*
    (sgp :v nil)
    (record-history "retrieval-history")
    (setf *set-break* nil)) 
  nil)

(defun run-it ()
  (setf *set-break* t)
  
  (let ((p (add-post-event-hook 'filter)))
    (grouped-recall)
    (delete-event-hook p))
   ;; hack to work with single threaded mode when json package doesn't exist
  #-:single-threaded-act-r (let ((data (third (first (funcall (read-from-string "json::decode-json-from-string") (get-history-data "retrieval-history" nil 15565))))))
                             (sgp-fct (list :v *out-file*))
                             (model-output (fourth (second data))))
  #+:single-threaded-act-r (let ((data (third (first (get-history-data "retrieval-history" nil 15565)))))
                             (sgp-fct (list :v *out-file*))
                             (model-output (fourth (second data))))
  )

(load-act-r-code "ACT-R:tutorial;lisp;grouped.lisp")
