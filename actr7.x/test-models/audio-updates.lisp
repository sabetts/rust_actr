
(defvar *audio-ms-times* nil)

(defun run-it ()
  (dolist (x '(nil t))
    (setf *audio-ms-times* x)
    (reset)
    (new-tone-sound 100 .1 .5)
    (new-tone-sound 1000 800 200 t)
    (let ((id1 (new-ongoing-sound "first" .2 .5))
          (id2 (new-ongoing-sound "other" 200 500 100 'external 'word  t '(:both both 10) '(:evt only-evt "'evt'") '(:sound only-sound "'snd'"))))
      (model-output "~%Times are in ~:[seconds~;milliseconds~]" *audio-ms-times*)
      (run 1)
      (print-audicon)
      (run 1)
      (print-audicon)
      (end-ongoing-sound id1)
      (schedule-event-relative .7 'end-ongoing-sound :params (list id2))
      
      (run 10)
      (print-audicon)
      (run 10)
      (print-audicon))))
    
  
  
(load-act-r-model "ACT-R:test-models;models;audio-updates.lisp")
