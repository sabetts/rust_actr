cd actr$1
svn update
rnum="$(svnversion)"
cd ..

cd $rnum

scp linux_standalone.zip db30@actr-server.hpc1.cs.cmu.edu:~/www/actr$1/next

cd ..
