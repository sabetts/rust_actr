start "ACT-R" apps/act-r.exe   --dynamic-space-size 4000 --end-runtime-options --no-sysinit --no-userinit --load set-logical.lisp --eval "(load-patch-files)" --eval "(init-des)" --eval "(echo-act-r-output)" --eval "(load-user-files)" --eval "(mp-print-versions)" --eval "(declaim (sb-ext:muffle-conditions SB-KERNEL:REDEFINITION-WITH-DEFUN))"

start "" "environment/Start Environment.exe"