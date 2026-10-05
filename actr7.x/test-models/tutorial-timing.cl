;;;  -*- mode: LISP; Syntax: COMMON-LISP;  Base: 10 -*-
;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;
;;; 
;;; Author      : Dan Bothell
;;; Copyright   : (c) 2026 Dan Bothell
;;; Availability: Covered by the GNU LGPL, see LGPL.txt
;;; Address     : Department of Psychology 
;;;             : Carnegie Mellon University
;;;             : Pittsburgh, PA 15213-3890
;;;             : db30@andrew.cmu.edu
;;; 
;;; 
;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;
;;; 
;;; Filename    : tutorial-timing.cl
;;; Version     : 1.0
;;; 
;;; Description : Code to run the tutorial tasks a large number of times to get
;;;             : the run times.
;;; 
;;; Bugs        : 
;;;
;;; To do       : 
;;; 
;;; ----- History -----
;;; 2026.02.05 Dan
;;;             : * Refined version of the code that was used to generate the
;;;             :   graphs for the 2024 and 2021 workshop slides.
;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;
;;;
;;; General Docs:
;;; 
;;; Runs all of the tutorial models through thier tasks multiple times to report
;;; a total run time or the run times for individual event actions as well as
;;; resetting.
;;; 
;;; It will force the trace to be off (regardless of the model setting), ignore
;;; the real time value for the run functions (forced to simulated time), force
;;; the experiment windows to always be virtual, and use a fixed seed value for
;;; the model.
;;;
;;; The corresponding total_times function in tutorial_timing.py can be used to
;;; get the Python timing for a total run time comparison.
;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;
;;;
;;; Public API:
;;;
;;; tutorial-total-time-results (name &optional fib)
;;; 
;;; Runs the tutorial models to record just the total run time for the tasks.
;;; Saves the results in a .csv file with the given name (string), and if fib
;;; is non-nil then it will also run the recursive fibonacci function first to
;;; try and get the processor up to speed.
;;;
;;; reset-and-running-results (name &optional fib)
;;;
;;; Similar to tutorial-total-time-results execept that it also records the
;;; time spent in the reset function and the run-sched-queue function (the 
;;; function that runs and executes the scheduled events when ACT-R is run by
;;; any of the ACT-R running functions).
;;;
;;; reset-running-and-event-results (name &optional tree fib)
;;;
;;; Similar to reset-and-running-results.  However, it now runs the tasks 
;;; four times.  The first time is to record all of the event actions that are
;;; performed.  The second is like reset-and-running-results.  The third is
;;; to record the number and total duration for each of the calls to an event
;;; action, and then the fourth is a repeat like reset-and-running-results.
;;; It also has the option of enabling the procedural parameter :use-tree to
;;; enable the decision tree in production matching.
;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;
;;;
;;; Design Choices:
;;; 
;;; For some tasks, use custom functions that will turn off some specific output
;;; that's not needed for timing purposes.
;;;
;;; Because some processors have variable clock speed for power efficiency,
;;; have an option to perform a recursive Fibonacci calculation first to try 
;;; and get it up to full speed before running the models.
;;;
;;; The number of times to run each model was chosen so that they all take about
;;; the same duration to run, but that varies by machine and OS so it's not 
;;; always going to work out the same.
;;; 
;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;
;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;
;;; 
;;; The code
;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;

#+:packaged-actr (in-package :act-r)
#+(and :clean-actr (not :packaged-actr) :ALLEGRO-IDE) (in-package :cg-user)
#-(or (not :clean-actr) :packaged-actr :ALLEGRO-IDE) (in-package :cl-user)

(require-compiled "time-functions")


;; A simple module to turn off :v and :cmdt after a model is reset

(defun kill-v (module)
               (declare (ignore module))
               (sgp :v nil :cmdt nil))


(define-module kill-v nil nil :version "1" :documentation "Set v to nil" :reset (nil nil kill-v) :required t)

;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;


;; Redefine open-exp-window to force it virtual, and the running 
;; functions to ignore the real-time flag when provided.

(defvar *open-win* (fdefinition (read-from-string "open-exp-window")))
(defvar *run* (fdefinition (read-from-string "run")))
(defvar *run-full* (fdefinition (read-from-string "run-full-time")))
(defvar *run-until* (fdefinition (read-from-string "run-until-condition")))

(defun open-exp-window (&rest rest)
  (let ((p (position :visible rest)))
    (if p
        (progn
          (setf (nth (1+ p) rest) nil)
          (apply *open-win* rest))
      (apply *open-win* (append rest (list :visible nil))))))

(defun run (&rest rest)
  (funcall *run* (first rest)))

(defun run-full-time (&rest rest)
  (funcall *run-full* (first rest)))

(defun run-until-condition (&rest rest)
  (funcall *run-until* (first rest)))

;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;


;; For recording the event functions, create a list to hold all
;; of the functions called as events as recorded with a function
;; that will be used as an event hook.

(defvar *event-fns*)

(defun record-events (evt)
  (when (symbolp (evt-action evt))
    (pushnew (evt-action evt) *event-fns*)))

;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;

;; Fibonacci function used to try to get the processor up to speed.

(defun fib (n)
  (if (<= n 1) 1 (+ (fib (1- n)) (fib (- n 2)))))

;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;




;; Given a function to run and a list of parameters
;; to pass it:
;; - run the function once to record all the event
;;   functions that are called
;; - run it again without the event hook recording
;;   the time to run the given function as well as
;;   run-sched-queue (internals of all running fns)
;;   and reset
;; - run it again recording the times for all the
;;   event fns
;; - repeat the second run (coarse times)
;;
;; Returns the list with the lists of times for the
;; last three runs.
;; All runs are performed with a fixed ACT-R seed
;; (if set in the model that's used otherwise it's
;; (101 1), and if the optional parameter is true 
;; then the :use-tree flag is set (unless explicitly
;; unset in the model).

(defun event-timing-model (run-fn params &optional with-tree run-fib)
  (let ((results nil)
        (hook (add-post-event-hook 'record-events)))
    
    ;; Set a fixed seed and whether to use the tree extension
    
    (if with-tree
        (ssp :starting-parameters (:seed (101 1) :use-tree t :model-warnings nil))
      (ssp :starting-parameters (:seed (101 1) :model-warnings nil)))
           
    ;; Run once to record the possible events
    
    (setf *event-fns* nil)
    (apply run-fn params)
    (delete-event-hook hook)
        
    ;; Remove the tree data so that the first reset
    ;; has to rebuild it, and do that directly for
    ;; simplicity (could break with changes to procedural
    ;; module) and assume no remote connection is 
    ;; accessing procedural information at this time.
    
    (when with-tree
      (setf (procedural-conflict-tree (get-module procedural)) nil)
      (setf (procedural-last-conflict-tree  (get-module procedural)) nil))
    
    (when run-fib
      (fib 45))
  
    ;; Run to get overall timing for only the task and coarse ACT-R components
    
    (push (list :before
                (eval 
                 `(ctime-functions (,run-fn run-sched-queue reset) 
                                   (apply ',run-fn ',params)))) results)
   
    ;; Run to record just the events which will start with the
    ;; tree data already in place if enabled
    
    (push (list :events
                (eval 
                 `(ctime-functions ,*event-fns*
                                   (apply ',run-fn ',params)))) results)
    
    ;; Run again to get overall timing for the task and coarse ACT-R components
    ;; Removing the tree data again for consistency 
    
    (when with-tree
      (setf (procedural-conflict-tree (get-module procedural)) nil)
      (setf (procedural-last-conflict-tree  (get-module procedural)) nil))
    
    (push (list :after
                (eval 
                 `(ctime-functions (,run-fn run-sched-queue reset)
                                  (apply ',run-fn ',params)))) results)
    results))

;; Don't use the tree, and just run for basic timing
;; after running a dummy task to 'warm up' the CPU to get
;; the total, running, and reset times.


(defun simple-times-model (run-fn params &optional fib)
  
  ;; Set a fixed seed and turn off warnings 
  
  (ssp :starting-parameters (:seed (101 1) :model-warnings nil))
       
  ;; Run a big fib to warm up if requested

  (when fib
    (fib 45))
    
  (eval 
   `(ctime-functions (,run-fn run-sched-queue reset) 
                     (apply ',run-fn ',params))))

;; simple timing of the task

(defun only-total-time (run-fn params &optional fib)
  
  ;; Set a fixed seed and turn off warnings 
  
  (ssp :starting-parameters (:seed (101 1) :model-warnings nil))
  
  ;; Run a big fib to warm up if requested
  
  (when fib
    (fib 45))
  
  ;; just run once for total task time
  
  (third (first (eval 
		 `(ctime-functions (,run-fn) 
				   (apply ',run-fn ',params))))))


;; Get count of chunks created in a single run

(defun chunk-counts (run-fn params)
  
  (ssp :starting-parameters (:seed (101 1) :model-warnings nil))
    
  (let ((start (length (no-output (chunks)))))
    (apply run-fn params)
    (- (length (no-output (chunks))) start)))

;; Some stub functions for running the tutorial
;; models that either don't have a task function or
;; need to have it adjusted in some way

;; Models, like unit 1, that just need to be reset
;; and then run for some length of time.

(defun just-run (n time)
  (dotimes (i n)
    (reset)
    (run time)))

;; For tasks that don't have a multiple run
;; option function.

(defun exp-runner (n exp &rest r)
  (dotimes (i n)
    (apply exp r)))

;; Kill the output for the subitize task

(defun subitize-runner (n)
  (defun report-data (x) x)
  (dotimes (i n) (subitize-experiment)))

;; kill the output for the sperling task

(defun sperling-runner (n)
  (setf *show-responses* nil)
  (sperling-experiment n))

;; kill the output from the fan tasks

(defun fan-runner (n)
  (defun output-person-location (x) x)
  (dotimes (i n)
    (fan-experiment)))

;; kill the output for past-tense

(defun past-tense-runner (n exp &rest r)
  (defun rep-f-i (start end count) (declare (ignore start end count)) nil)
  (defun print-header () nil)
  (dotimes (i n)
    (apply exp r)))


;; List of tutorial models with the file(s) that need to be loaded and the details
;; of the function to run for a fixed number of times.
;; It assumes that the reference solution files are in the examples/act-r-solutions
;; directory.
;; The number of runs were chosen so each takes about the same time to complete
;; in the current system.

(defvar *model-run-lists* 
    '((1 addition 83000 ("ACT-R:tutorial;unit1;addition.lisp") (just-run (10)))
      (1 count 97000 ("ACT-R:tutorial;unit1;count.lisp") (just-run (10)))
      (1 semantic 75000 ("ACT-R:tutorial;unit1;semantic.lisp") (just-run (10)))
                
      (2 demo2 48000 ("ACT-R:tutorial;lisp;demo2.lisp") (exp-runner (demo2-experiment)))
      (2 unit2 33000 ("ACT-R:tutorial;lisp;unit2.lisp" "ACT-R:examples;act-r-solutions;unit-2-solution-model.lisp") (exp-runner (unit2-experiment)))
                
      (3 sperling 4000 ("ACT-R:tutorial;lisp;sperling.lisp") (sperling-runner ()))
      (3 subitize 2000 ("ACT-R:tutorial;lisp;subitize.lisp" "ACT-R:examples;act-r-solutions;unit-3-subitize-solution-model.lisp") (subitize-runner ()))
                
      (4 paired 500 ("ACT-R:tutorial;lisp;paired.lisp") (paired-experiment ()))
      (4 zbrodoff 50 ("ACT-R:tutorial;lisp;zbrodoff.lisp" "ACT-R:examples;act-r-solutions;unit-4-zbrodoff-solution-model.lisp") (zbrodoff-compare ()))
                
      (5 fan-no-pm 1400 ("ACT-R:tutorial;lisp;fan-no-pm.lisp") (fan-runner ()))
      (5 fan 800 ("ACT-R:tutorial;lisp;fan.lisp") (fan-runner ()))
      (5 siegler 4600 ("ACT-R:tutorial;lisp;siegler.lisp") (siegler-experiment ()))
      (5 one-hit 1200 ("ACT-R:tutorial;lisp;onehit.lisp" "ACT-R:examples;act-r-solutions;unit-5-1hit-blackjack-solution-model.lisp") (onehit-learning (nil)))
      
      (6 bst 450 ("ACT-R:tutorial;lisp;bst.lisp") (bst-experiment ()))
      (6 choice 1700 ("ACT-R:examples;act-r-solutions;unit-6-choice-solution-task.lisp" "ACT-R:examples;act-r-solutions;unit-6-choice-solution-model.lisp") (choice-data ()))
      (6 choice-event-task 2000 ("ACT-R:examples;act-r-solutions;unit-6-choice-solution-event-based-task.lisp" "ACT-R:examples;act-r-solutions;unit-6-choice-solution-model.lisp") (choice-data ()))
      
      (7 paired-learning 125 ("ACT-R:tutorial;lisp;paired.lisp" "ACT-R:tutorial;unit7;paired-learning-model.lisp") (paired-experiment ()))
      (7 past-tense 7 ("ACT-R:tutorial;lisp;past-tense.lisp" "ACT-R:examples;act-r-solutions;unit-7-past-tense-solution-model.lisp") (past-tense-runner (past-tense-trials 20000)))
      
      (8 bst-ppm 400 ("ACT-R:tutorial;lisp;bst-ppm.lisp") (bst-ppm-experiment ()))
      (8 categorize 2800 ("ACT-R:tutorial;lisp;categorize.lisp" "ACT-R:examples;act-r-solutions;unit-8-categorize-solution-model.lisp") (categorize-fct (nil nil)))))


;; Function to run the tutorial models each a fixed number of times
;; to just get total time.
;; It will save the results in a .csv file in the ACT-R directory
;; with the name given.  The columns will be:
;; unit #, model name, how many runs, total time
;;

(defun tutorial-total-time-results (name &optional fib)
  (let (data)

    (dolist (x *model-run-lists*)
      
      (let ((success t))
        
        (dolist (y (fourth x))
          (if (probe-file (translate-logical-pathname y))
              (actr-load y)
            (setf success nil)))
        (if success
            (progn
              (format t "Running ~s~%" (second x))
              (push (list (first x) (second x) (third x) (/ (apply 'only-total-time (let ((vals (fifth x))) (list (first vals) (append (list (third x)) (second vals)) fib))) internal-time-units-per-second)) data))
          (format t "Failed to load for model run ~s~%" x))))
    
    (with-open-file (f (translate-logical-pathname (format nil "ACT-R:~a.csv" name)) :direction :output :if-does-not-exist :create)
      (format f "unit,model,runs,time~%")
      (dolist (y (reverse data))
        (format f "~a,~a,~d,~f~%" (first y) (second y) (third y) (fourth y))))
    data))


(defun reset-and-running-results (name &optional fib)
  (let (data)

    (dolist (x *model-run-lists*)
      
      (let ((success t))
        
        (dolist (y (fourth x))
          (if (probe-file (translate-logical-pathname y))
              (actr-load y)
            (setf success nil)))
        (if success
            (progn
              (format t "Running ~s~%" (second x))
              (push (list (first x) (second x) (third x) (reverse (apply 'simple-times-model (let ((vals (fifth x))) (list (first vals) (append (list (third x)) (second vals)) fib))))) data))
          (format t "Failed to load for model run ~s~%" x))))
    
    (with-open-file (f (translate-logical-pathname (format nil "ACT-R:~a.csv" name)) :direction :output :if-does-not-exist :create)
      (format f "unit,model,runs,total time,proportion reset time,proportion running schedule,# resets, total reset time, time/reset,# run-sched-queue,total schedule running time,time/schedule_run~%")
      (dolist (y (reverse data))
        (let* ((data (fourth y))
               (total-time (/ (third (first data)) internal-time-units-per-second))
               (sched-count (second (second data)))
               (sched-time (/ (third (second data)) internal-time-units-per-second))
               (reset-count (second (third data)))
               (reset-time (/ (third (third data)) internal-time-units-per-second)))
          
          (format f "~a,~a,~d,~f,~f,~f,~d,~f,~f,~d,~f,~f~%" (first y) (second y) (third y) 
            total-time (/ reset-time total-time) (/ sched-time total-time)
            reset-count reset-time (/ reset-time reset-count)
            sched-count sched-time (/ sched-time sched-count)))))
    data))


(defun reset-running-and-event-results (name &optional tree fib)
  (let (data)

    (dolist (x *model-run-lists*)
      
      (let ((success t))
        
        (dolist (y (fourth x))
          (if (probe-file (translate-logical-pathname y))
              (actr-load y)
            (setf success nil)))
        (if success
            (progn
              (format t "Running ~s~%" (second x))
              (push (list (first x) (second x) (third x) (apply 'event-timing-model (let ((vals (fifth x))) (list (first vals) (append (list (third x)) (second vals)) tree fib)))) data))
          (format t "Failed to load for model run ~s~%" x))))
    
    (with-open-file (f (translate-logical-pathname (format nil "ACT-R:~a.csv" name)) :direction :output :if-does-not-exist :create)
      (format f "unit,model,runs,")
      (dolist (x '("before" "after" "average"))
        (dolist (y '("total time" "proportion reset time" "proportion running schedule" "# resets" "total reset time" "time/reset" "# run-sched-queue" "total schedule running time" "time/schedule_run"))
          (format f "~a(~a)," y x)))
            
      (let ((events (sort (remove-duplicates (mapcan (lambda (x) (mapcar 'car (cadr (assoc :events (fourth x))))) data)) 'string< :key 'symbol-name)))
        (format f "~{~a~^,~}~%" events)
        
        (dolist (y (reverse data))
          (let* ((data (fourth y))
                 (before (reverse (cadr (assoc :before data))))
                 (after (reverse (cadr (assoc :after data))))
                 (es (cadr (assoc :events data)))
                 (total-time-b (/ (third (first before)) internal-time-units-per-second))
                 (sched-count-b (second (second before)))
                 (sched-time-b (/ (third (second before)) internal-time-units-per-second))
                 (reset-count-b (second (third before)))
                 (reset-time-b (/ (third (third before)) internal-time-units-per-second))
                 (total-time-a (/ (third (first after)) internal-time-units-per-second))
                 (sched-count-a (second (second after)))
                 (sched-time-a (/ (third (second after)) internal-time-units-per-second))
                 (reset-count-a (second (third after)))
                 (reset-time-a (/ (third (third after)) internal-time-units-per-second)))
            
            (format f "~a,~a,~d," (first y) (second y) (third y))
          
            (format f "~f,~f,~f,~d,~f,~f,~d,~f,~f," 
              total-time-b (/ reset-time-b total-time-b) (/ sched-time-b total-time-b)
              reset-count-b reset-time-b (/ reset-time-b reset-count-b)
              sched-count-b sched-time-b (/ sched-time-b sched-count-b))
            
            (format f "~f,~f,~f,~d,~f,~f,~d,~f,~f," 
              total-time-a (/ reset-time-a total-time-a) (/ sched-time-a total-time-a)
              reset-count-a reset-time-a (/ reset-time-a reset-count-a)
              sched-count-a sched-time-a (/ sched-time-a sched-count-a))
            
            (format f "~f,~f,~f,~d,~f,~f,~d,~f,~f," 
              (/ (+ total-time-b total-time-a) 2)
              (/ (+ reset-time-b reset-time-a) (+ total-time-b total-time-a))
              (/ (+ sched-time-b sched-time-a) (+ sched-time-b total-time-a))
              (/ (+ reset-count-b reset-count-a) 2)
              (/ (+ reset-time-b reset-time-a) 2)
              (/ (+ reset-time-b reset-time-a) (+ reset-count-b reset-count-a))
              (/ (+ sched-count-b sched-count-a) 2)
              (/ (+ sched-time-b sched-time-a) 2)
              (/ (+ sched-time-b sched-time-a) (+ sched-count-b sched-count-a)))
            
            (dolist (z events)
              (let ((vals (assoc z es)))
                (format f "~f," (if vals (/ (third vals) (second vals)) 0))))
            (format f "~%")))))
    
    data))

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


