
actr_version=actr$1

cd $actr_version/$actr_version

r_ver=`svnversion`

cd ..
cd $r_ver

scp distro/* db30@act-r.psy.cmu.edu:~/www/$actr_version/next

scp ?_$actr_version.zip db30@act-r.psy.cmu.edu:~/www/$actr_version/next







