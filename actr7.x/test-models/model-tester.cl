;;;  -*- mode: LISP; Syntax: COMMON-LISP;  Base: 10 -*-
;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;
;;; 
;;; Author      : Dan Bothell
;;; Copyright   : (c) 2007 Dan Bothell
;;; Availability: Covered by the GNU LGPL, see LGPL.txt
;;; Address     : Department of Psychology 
;;;             : Carnegie Mellon University
;;;             : Pittsburgh, PA 15213-3890
;;;             : db30@andrew.cmu.edu
;;; 
;;; 
;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;
;;; 
;;; Filename    : model-tester.cl
;;; Version     : 5.0
;;; 
;;; Description : Runs test models and compares the output to reference runs.
;;; 
;;; Bugs        : 
;;;
;;; To do       : 
;;; 
;;; ----- History -----
;;; 
;;; 07.10.11 Dan
;;;             : * Finally decided to start automating the version testing...
;;; 07.10.12 Dan
;;;             : * Added tests to strip off linefeed or return characters from
;;;             :   the reference files so that a single ref file can be used
;;;             :   to test regardless of the native format for the output.
;;; 07.12.20 Dan
;;;             : * Adding a function to run and test a single file.
;;; 2012.06.13 Dan
;;;             : * Added an ignore-errors to run-actr-tests so that it'll
;;;             :   still run others if it encounters an error.
;;; 2012.08.07 Dan
;;;             : * Since the production warnings will go to *error-output* when
;;;             :   *one-stream-hack* is set just ignore *one-stream-hack* while 
;;;             :   doing test runs since the two streams will not be the same
;;;             :   in this case on purpose.
;;; 2013.08.08 Dan
;;;             : * Changed run-actr-tests so that it returns a list of the 
;;;             :   tests that didn't match.
;;; 2013.12.05 Dan
;;;             : * Fixed a bug with compare-test-model-files so it returns t
;;;             :   for all conditions which indicate a mismatch.
;;; 2014.01.27 Dan
;;;             : * Last update changed it so that only the first different line
;;;             :   was output, but I prefer to see all the differences.
;;; 2014.05.30 Dan [2.0]
;;;             : * Now there's an optional parameter to run-actr-tests which if
;;;             :   true will print out only the "edit" differences between the
;;;             :   reference and output files instead of every different line
;;;             :   which much better handles things like an extra line in the
;;;             :   output.  However, it's costly to perform and requires a lot
;;;             :   of memory to read in and compare the files.
;;; 2014.06.02 Dan
;;;             : * Instead of doing everything in memory for the edit distance
;;;             :   use file-position to read data from the files as needed and
;;;             :   write the matrix out to a .dat file as it goes.  Much slower,
;;;             :   but doesn't run into heap size issues with larger output files.
;;;             : * Added the option of writing a "diff like" file to the test 
;;;             :   functions.  If the second optional parameter is t then it 
;;;             :   writes a file with a .dif extension to the test-models dir
;;;             :   with the union of the two files where differences are noted
;;;             :   in the first character position followed by a : and a space.
;;;             :   If the lines are the same there's a space, if it's only in 
;;;             :   the .out file there's a +, if it's only in the .ref there's
;;;             :   a -, and if there's a difference between the two then it prints
;;;             :   the .ref file's line first with a < and then the .out file's
;;;             :   line with a >.
;;; 2014.06.03 Dan
;;;             : * Use an approximation for the edit distance which greatly
;;;             :   reduces the storage necessary to reconstruct things by only
;;;             :   considering the area between the diagonals +5 to either side
;;;             :   for the large files.
;;;             :   The assumption being that the differences will mostly be
;;;             :   transforms (the main diagonal) with a few additions and/or
;;;             :   deletions.  
;;; 2014.06.05 Dan
;;;             : * Scrap my approximation and use the Hirschberg algorithm to
;;;             :   align the files.
;;; 2014.06.12 Dan
;;;             : * Add the function save-actr-tests which dumps the log to a
;;;             :   file instead of printing it in the <listener>.
;;; 2014.08.12 Dan [3.0]
;;;             : * Instead of copying the tutorial models here and tooling them
;;;             :   up with extra functions and parameters have a stub file in
;;;             :   the test directory which loads the tutorial model directly.
;;;             : * To handle setting any parameters in a model loaded from
;;;             :   elsewhere include a testing module which can use the reset
;;;             :   function to re-write parameters in the third reset function.
;;;             :   The only down side is that output generated by the model
;;;             :   definition will not get shunted to the output file since the
;;;             :   :v and :cmdt parameters won't be set to *out-file* until
;;;             :   after the definition code is evaled, but that shouldn't
;;;             :   matter for the tutorial and example models anyway (the ones
;;;             :   which will require overriding the parameter settings).
;;; 2014.08.13 Dan
;;;             : * When running a test automatically set the :testing-function 
;;;             :   to a function which sets :v and :cmdt to *out-file* before
;;;             :   loading the file.  If the file needs to set other things
;;;             :   then it will need to change the value of :testing-function.
;;;             :   After running the test the :testing-function is then set
;;;             :   back to nil before returning. 
;;; 2014.08.18 Dan
;;;             : * Added the macro force-virtual-window which temporarily rebinds 
;;;             :   open-exp-window to a function that always opens a virtual
;;;             :   window.
;;;             : * Added the macro ignore-real-time which temporarily rebinds
;;;             :   run to a function that ignores the real-time flag.
;;; 2014.09.04 Dan
;;;             : * When restoring the :v and :cmdt parameters need to make sure
;;;             :   that there is a current meta-process.
;;; 2014.09.05 Dan
;;;             : * Clip trailing spaces, newlines, and carriage returns from
;;;             :   the lines to be compared.
;;; 2015.07.28 Dan
;;;             : * Changed the logicals to ACT-R.
;;; 2015.07.31 Dan
;;;             : * To solve an issue with ACL needing to load some extra
;;;             :   libraries mid-test and causing trouble just do a dummy
;;;             :   broadcast stream in run-actr-tests.
;;; 2016.03.08 Dan
;;;             : * Added a test to run-actr-test because if it was used on its
;;;             :   own and the file specified the .lisp part of the name it
;;;             :   could overwrite the original with an empty file.
;;; 2016.11.17 Dan
;;;             : * Can't use meta-process-names or with-meta-process.
;;; 2017.12.19 Dan
;;;             : * Removed the unnecessary *one-stream-hack* setting and the
;;;             :   commented out code for multiple meta-processes.
;;; 2017.12.20 Dan
;;;             : * Fixed the ignore-real-time and force-virtual-window macros
;;;             :   for the current definitions with optionals instead of keywords.
;;; 2018.01.02 Dan [4.0]
;;;             : * Reworked it to monitor the output streams so that it can
;;;             :   capture warnings and general output without having to catch
;;;             :   them in the run-it functions by using the *out-file* variable.
;;;             : * Added a clear-all into the testing fn to make sure each one
;;;             :   starts off clean.
;;; 2018.01.04 Dan
;;;             : * Add a remote version of the testing function for use by the
;;;             :   Python tester since there's no need to rewrite things.
;;;             : * Allow the testing-function to be set to a dispatced command.
;;;             : * Set :starting-parameters back to nil instead of a list that
;;;             :   sets :testing-function to nil.
;;; 2018.06.21 Dan
;;;             : * Force-virtual-window needs to use the keyword params now.
;;; 2018.06.22 Dan
;;;             : * Actually use force-virtual-window and ignore-real-time in
;;;             :   run-actr-test.
;;;             : * Can't use ignore-real-time since there's a real-time test
;;;             :   model. 
;;; 2019.06.18 Dan
;;;             : * Add a hack for ccl on the open call so that the file can be
;;;             :   used from different threads.
;;; 2020.01.13 Dan [4.1]
;;;             : * Removed the lambdas from the module interface.
;;; 2021.09.14 Dan
;;;             : * Changed how the pathnames are created for the remote compare
;;;             :   since underscores in the pathname seemed to be a problem for
;;;             :   translate-logical-pathname (at least in ACL).
;;; 2021.10.20 Dan [5.0]
;;;             : * Mark the module as required.
;;; 2023.05.25 Dan 
;;;             : * Sort the list of test models when using run-actr-tests so
;;;             :   that the problematic extras can be named so that they run
;;;             :   last.
;;; 2023.06.29 Dan
;;;             : * Turn off pretty printing when running the tests so that
;;;             :   outputs are more consistent.
;;;             : * Instead of directly passing the string to format use a ~a
;;;             :   in record-test-output.
;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;
;;;
;;; General Docs:
;;; 
;;; This provides a mechanims for comparing the outputs of models to a fixed
;;; reference run of that model.  The primary usage is for testing the public
;;; distributions of ACT-R to ensure they run the test models the same as the
;;; reference version across different Lisps/OSs.
;;;
;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;
;;;
;;; Public API:
;;;
;;; run-actr-test
;;;
;;; This function takes one required parameter, which should be a string naming a 
;;; file in the ACT-R:test-models directory which has a "lisp" file type, and two
;;; optional parameters.  If a file with that name existes it will load that file.
;;; It will create a new file with the same name but of type "out" in that directory
;;; overwriting such a file if it already exists. It will then bind the variable 
;;; *out-file* to an output stream for that file and then call the run-it function.
;;; The output stream will then be closed.
;;;
;;; After that, if there is another file with that name of type "ref" the "out"
;;; file is compared to the "ref" file.  If the first optional paramter is t
;;; then the comparison is done using the edit distance on the lines of the files.
;;; If the first optional parameter is nil or not given then the files are
;;; compared only line to line.  
;;;
;;; If there are any differences then the differences are printed to 
;;; *standard-output*.  If no differences are found a message is printed to 
;;; *standard-output* indicating that.  
;;;
;;; If the first and second optional parameters are true then an additional file
;;; with a type of "dif" is writen to the test-models directory (overwritting
;;; an existing file if there is one) which is the union of the "ref" and "out" 
;;; where each line starts with three extra characters: <c>, :, and space.  
;;; <c> will be a space if the files are the same at that line, a - if it only  
;;; appears in the "ref" file, a + if it only appears in the "out" file, and if
;;; the lines are different between the two (a replace operation in the edit actions)
;;; then both lines are output where the "ref" file's line has < and the "out"
;;; file's line has >.
;;;
;;; It returns t if the ref and out files are different and nil if they are the
;;; same or there is no ref file.
;;;
;;;
;;; run-actr-tests
;;;
;;; This function takes two optional parameters.  It effectively calls run-actr-test
;;; for each file in the ACT-R:test-models directory with a "lisp" file type 
;;; passing the optional parameters it is given.
;;;
;;; It returns a list of the files for which the "out" file did not match the 
;;; "ref" file.
;;; 
;;; save-actr-tests
;;; 
;;; Takes one required parameter and two optional parameters.  The required 
;;; parameter should be a pathname.  That file is opened for output and superceded
;;; if it exists and *standard-output* and *error-output* are then bound to that
;;; output stream.  The optional parameters are then passed to run-actr-tests.
;;;
;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;
;;;
;;; Design Choices:
;;; 
;;; The assumptions are that the models to be tested set the :v (and possibly
;;; :cmdt) parameter to the value of *out-file*, specify an explicit seed for
;;; the model, only use the act-r-random function to guarantee repeatability,
;;; and specify a function called run-it that is used to generate the output 
;;; for comparison.  If the model has any output other than the trace which is
;;; to appear in the file for comparison then the appropriate measures should 
;;; be taken to have that output end up in the comparison file (like using model-
;;; output or command-output for instance).
;;;
;;; 
;;; Note that the solution models and thier reference runs are in a protected
;;; zip file which is only available under the same restrictions as the regular
;;; solution models.
;;;
;;; Now, it's also possible to set the parameter :set-testing-params with ssp to
;;; a function to call after a model definition so that one can set parameters
;;; for a model without having to edit it.
;;;
;;; For example, to test the addition model in unit 1 one could have a file
;;; like this:
;;;
;;; (clear-all)
;;; (defun set-addition-params ()
;;;    (sgp-fct (list :v *out-file* :cmdt *out-file*)))
;;; (ssp :starting-parameters (:testing-function set-addition-params))
;;; (defun run-it () (run 10))
;;; (load "ACT-R:tutorial;unit1;addition.lisp")
;;;
;;; To make that even easier there's a function called set-testing-traces
;;; which will set :v and :cmdt like that.  Which means that this would be 
;;; sufficient:
;;; 
;;; (clear-all)
;;; (ssp :starting-parameters (:testing-function set-testing-traces))
;;; (defun run-it () (run 10))
;;; (load "ACT-R:tutorial;unit1;addition.lisp")
;;; 
;;;
;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;
;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;
;;; 
;;; The code
;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;

#+:packaged-actr (in-package :act-r)
#+(and :clean-actr (not :packaged-actr) :ALLEGRO-IDE) (in-package :cg-user)
#-(or (not :clean-actr) :packaged-actr :ALLEGRO-IDE) (in-package :cl-user)

(require-compiled "HIRSCHBERG" "ACT-R-support:hirschberg-diff-calculation")

(defvar *out-file*)

(defun run-actr-tests (&optional short-compare diff-file)
  #+:allegro (let ((*standard-output* (make-broadcast-stream *standard-output*))))
  (let ((mismatches))
    (dolist (file (sort (directory (translate-logical-pathname "ACT-R:test-models;*.lisp")) 'string< :key 'pathname-name) mismatches)
      (when (run-actr-test (pathname-name file) short-compare diff-file)
        (push (pathname-name file) mismatches)))))

(defun save-actr-tests (path &optional short-compare diff-file)
  (with-open-file (f path :direction :output :if-exists :supersede :if-does-not-exist :create)
    (let ((*standard-output* f)
          (*error-output* f))
      (run-actr-tests short-compare diff-file))))

(defun record-test-output (string)
  (format *out-file* "~a" string))

(add-act-r-command "Model-tester-output-function" 'record-test-output "Internal command for saving model output to a file for testing. Do not call." t)

(defmacro force-virtual-window (&body body)
  (let ((save (gensym)))
    `(let ((,save (fdefinition 'open-exp-window)))
       (unwind-protect 
           (progn
             (setf (fdefinition 'open-exp-window)
               (lambda (title &key (visible nil) (height 300) (width 300) (x 300) (y 300))
                 (declare (ignore visible))
                 (funcall ,save title :visible nil :height height :width width :x x :y y)))
             (progn ,@body))
         (setf (fdefinition 'open-exp-window) ,save)))))

(defmacro ignore-real-time (&body body)
  (let ((save (gensym)))
    `(let ((,save (fdefinition 'run)))
       (unwind-protect 
           (progn
             (setf (fdefinition 'run)
               (lambda (&rest rest)
                 (funcall ,save (first rest))))
             (progn ,@body))
         (setf (fdefinition 'run) ,save)))))
             


(defun run-actr-test (file-name &optional short-compare diff-file)
  (if (null (pathname-type file-name))
      (let ((*print-pretty* nil)
            (file (translate-logical-pathname (format nil "ACT-R:test-models;~a.lisp" file-name)))
            (base-pathname (translate-logical-pathname (format nil "ACT-R:test-models;~a" file-name))))
        
        (format t "~%###########################~%Testing file ~s~%" (pathname-name file))
        (let (on?)
          
          (with-open-file (f (merge-pathnames base-pathname (make-pathname :type "out")) :direction :output :if-exists :supersede :if-does-not-exist :create #+:ccl :sharing #+:ccl :external)
            (setf *out-file* f)
            (clear-all)
            (ssp :starting-parameters (:testing-function set-testing-traces))
            (load file)
            (terpri)
            (setf on? (turn-off-act-r-output))
            (monitor-act-r-command "model-trace" "Model-tester-output-function")
            (monitor-act-r-command "command-trace" "Model-tester-output-function")
            (monitor-act-r-command "warning-trace" "Model-tester-output-function")
            (monitor-act-r-command "general-trace" "Model-tester-output-function")
            (force-virtual-window (run-it))
            (ssp :starting-parameters nil)
            (remove-act-r-command-monitor "model-trace" "Model-tester-output-function")
            (remove-act-r-command-monitor "command-trace" "Model-tester-output-function")
            (remove-act-r-command-monitor "warning-trace" "Model-tester-output-function")
            (remove-act-r-command-monitor "general-trace" "Model-tester-output-function"))
          (prog1 
              (if (probe-file (merge-pathnames base-pathname (make-pathname :type "ref")))
                  (or (and (null short-compare) (compare-test-model-files base-pathname))
                      (and short-compare (compare-test-model-file-distance base-pathname diff-file)))
                (format t "File does not have a reference for comparison~%"))
            (when on? (echo-act-r-output)))))
    (format t "Invalid file for testing ~s.  Name provided must not include a type." file-name)))

  
(defun compare-test-model-files (file)
  (with-open-file (f1 (merge-pathnames file (make-pathname :type "ref")) :direction :input)
    (with-open-file (f2 (merge-pathnames file (make-pathname :type "out")) :direction :input)
      (let ((diff nil))
        (loop 
         (let ((l1 (read-line f1 nil :file-ended))
               (l2 (read-line f2 nil :file-ended)))
           
           (when (stringp l1) (setf l1 (string-right-trim '(#\space #\return #\linefeed) l1)))
           (when (stringp l2) (setf l2 (string-right-trim '(#\space #\return #\linefeed) l2)))
           
          (cond ((and (eq l1 :file-ended) (eq l2 :file-ended))
                 (unless diff
                   (format t "No differences detected.~%"))
                 (return diff))
                ((eq l1 :file-ended)
                 (format t "Reference file shorter than output file.~%")
                 (format t "First extra line of output file is:~%~S~%" l2)
                 (return t))
                ((eq l2 :file-ended)
                 (format t "Output file shorter than reference file.~%")
                 (format t "First extra line of reference file is:~%~S~%" l1)
                 (return t))
                ((not (string-equal l1 l2))
                 (setf diff t)
                 (format t "Files differ.~%Reference file: ~s~%Output    file: ~S~%" l1 l2)))))))))

                      
                      
(defun compare-test-model-file-distance (file &optional write-diff)
  (let ((lines (make-array (list 0) :adjustable t :fill-pointer t :element-type 'string))
        (ref-indexes (make-array (list 0) :adjustable t :fill-pointer t :element-type 'fixnum))
        (out-indexes (make-array (list 0) :adjustable t :fill-pointer t :element-type 'fixnum)))
    
    (with-open-file (ref-file (merge-pathnames file (make-pathname :type "ref")) :direction :input)
      (loop (let ((l (read-line ref-file nil :file-ended)))
              (when (eq l :file-ended) (return))
              (let* ((nl (string-right-trim '(#\space #\return #\linefeed) l))
                     (pos (position nl lines :test 'string-equal)))
                (if pos
                    (vector-push-extend pos ref-indexes)
                  (progn 
                    (vector-push-extend (fill-pointer lines) ref-indexes)
                    (vector-push-extend nl lines)))))))
    
    (with-open-file (out-file (merge-pathnames file (make-pathname :type "out")) :direction :input)
      (loop (let ((l (read-line out-file nil :file-ended)))
              (when (eq l :file-ended) (return))
              (let* ((nl (string-right-trim '(#\space #\return #\linefeed) l))
                     (pos (position nl lines :test 'string-equal)))
                (if pos
                    (vector-push-extend pos out-indexes)
                  (progn 
                    (vector-push-extend (fill-pointer lines) out-indexes)
                    (vector-push-extend nl lines)))))))
    
    (multiple-value-bind (z w)
        (hirschberg ref-indexes out-indexes :strip-ends t :test '= :type 'fixnum :missing -1 :coerce nil)
      (if (equalp z w)
          (progn 
            (format t "No differences detected.~%") 
            nil)
        (progn
          (dotimes (i (length z))
            (cond ((= (elt z i) (elt w i))
                   ;ignore same
                   )
                  ((= (elt z i) -1)
                   (format t "~%Extra line    : ~a~%" (elt lines (elt w i))))
                  ((= (elt w i) -1)
                   (format t "~%Missing line  : ~a~%" (elt lines (elt z i))))
                  (t ; a transform
                   (format t "~%Reference file: ~a~%" (elt lines (elt z i)))
                   (format t "Output file   : ~a~%" (elt lines (elt w i))))))
          (when write-diff
            (with-open-file (f (merge-pathnames file (make-pathname :type "dif")) 
                               :direction :output :if-exists :supersede :if-does-not-exist :create)
              (dotimes (i (length z))
                (cond ((= (elt z i) (elt w i))
                       (format f " ~a~%" (elt lines (elt z i))))
                      ((= (elt z i) -1)
                       (format f "+~a~%" (elt lines (elt w i))))
                      ((= (elt w i) -1)
                       (format f "-~a~%" (elt lines (elt z i))))
                      (t ; a transform
                       (format f "<~a~%" (elt lines (elt z i)))
                       (format f ">~a~%" (elt lines (elt w i))))))))
          t)))))


(defstruct testing-module function)

(defun testing-module-params (m p)
  (if (consp p)
      (case (car p)
        (:testing-function (setf (testing-module-function m) (cdr p))))
    (case p
      (:testing-function (testing-module-function m)))))
    
    
(defun call-testing-module-function (m)
  (awhen (testing-module-function m)
         (dispatch-apply it)))

(defun create-testing-module (x)
  (declare (ignore x)) 
  (make-testing-module))


(define-module-fct 'testing-module nil 
  (list (define-parameter :testing-function :valid-test 'local-or-remote-function-or-nil :default-value nil :warning "Function or nil" 
          :documentation "Function to call to set parameters in a model for testing purposes."))
  :creation 'create-testing-module
  :params 'testing-module-params
  :reset (list nil nil 'call-testing-module-function)
  :required t
  :version "5.0"
  :documentation "Module to make testing models easier.")



(defun set-testing-traces ()
  (sgp :v t :cmdt t))
    



(defun remote-test-model-file-distance (file)
  (let ((ref-pathname (merge-pathnames (format nil "~a.ref" file) (translate-logical-pathname "ACT-R:test-models;")))
        (out-pathname (merge-pathnames (format nil "~a.out" file) (translate-logical-pathname "ACT-R:test-models;"))))
    
    (unless (and (probe-file ref-pathname) (probe-file out-pathname))
      (act-r-output "One of the files for comparison does not exist.")
      (return-from remote-test-model-file-distance t))
    
    (let ((lines (make-array (list 0) :adjustable t :fill-pointer t :element-type 'string))
          (ref-indexes (make-array (list 0) :adjustable t :fill-pointer t :element-type 'fixnum))
          (out-indexes (make-array (list 0) :adjustable t :fill-pointer t :element-type 'fixnum)))
      
      (with-open-file (ref-file ref-pathname :direction :input)
        (loop (let ((l (read-line ref-file nil :file-ended)))
                (when (eq l :file-ended) (return))
                (let* ((nl (string-right-trim '(#\space #\return #\linefeed) l))
                       (pos (position nl lines :test 'string-equal)))
                  (if pos
                      (vector-push-extend pos ref-indexes)
                    (progn 
                      (vector-push-extend (fill-pointer lines) ref-indexes)
                      (vector-push-extend nl lines)))))))
      
      (with-open-file (out-file out-pathname :direction :input)
        (loop (let ((l (read-line out-file nil :file-ended)))
                (when (eq l :file-ended) (return))
                (let* ((nl (string-right-trim '(#\space #\return #\linefeed) l))
                       (pos (position nl lines :test 'string-equal)))
                  (if pos
                      (vector-push-extend pos out-indexes)
                    (progn 
                      (vector-push-extend (fill-pointer lines) out-indexes)
                      (vector-push-extend nl lines)))))))
      
      (multiple-value-bind (z w)
          (hirschberg ref-indexes out-indexes :strip-ends t :test '= :type 'fixnum :missing -1 :coerce nil)
        (if (equalp z w)
            (progn 
              (act-r-output "No differences detected.") 
              nil)
          (dotimes (i (length z) t)
            (cond ((= (elt z i) (elt w i))
                   ;ignore same
                   )
                  ((= (elt z i) -1)
                   (act-r-output "~%Extra line    : ~a" (elt lines (elt w i))))
                  ((= (elt w i) -1)
                   (act-r-output "~%Missing line  : ~a" (elt lines (elt z i))))
                  (t ; a transform
                   (act-r-output "~%Reference file: ~a" (elt lines (elt z i)))
                   (act-r-output "Output file   : ~a" (elt lines (elt w i)))))))))))

(add-act-r-command "testing-model-compare" 'remote-test-model-file-distance "Compare .ref and .out files given a file name in the ACT-R:test-models; directory and return t/nil to indicate mismatching and print differences. Params: name.")

#|
This library is free software; you can redistribute it and/or
modify it under the terms of the GNU Lesser General Public
License as published by the Free Software Foundation; either
version 2.1 of the License, or (at your option) any later version.

This library is distributed in the hope that it will be useful,
but WITHOUT ANY WARRANTY; without even the implied warranty of
MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU
Lesser General Public License for more details.

You should have received a copy of the GNU Lesser General Public
License along with this library; if not, write to the Free Software
Foundation, Inc., 59 Temple Place, Suite 330, Boston, MA  02111-1307  USA
|#
