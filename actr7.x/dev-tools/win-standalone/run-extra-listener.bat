start "Remote ACT-R" apps/act-r.exe   --dynamic-space-size 4000 --end-runtime-options --no-sysinit --no-userinit --eval "(make-package :remote :use (list \"COMMON-LISP\" \"COMMON-LISP-USER\"))" --eval "(in-package :remote)" --load "tutorial/lisp/remote-actr-connection.lisp" --load set-logical.lisp --eval "(exclude-model-trace)" --eval "(finish-output *error-output*)"

