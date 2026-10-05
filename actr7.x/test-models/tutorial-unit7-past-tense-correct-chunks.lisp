

(actr-load "ACT-R:tutorial;lisp;past-tense.lisp")

 
(defun run-it () 

  (setf *word-list* (make-word-freq-list *verbs*))
  (let ((have (cdr (find 'have *word-list* :key 'second)))
        (use (cdr (find 'use *word-list* :key 'second))))
    (sgp :model-warnings nil)
    (pprint-chunks-fct (define-chunks-fct 
                           (list 
                            (list 
                             'isa 'past-tense 'verb (first have) 'stem (second have) 'suffix (third have))
                             (list 
                              'isa 'past-tense 'verb (first use) 'stem (second use) 'suffix (third use)))))))
