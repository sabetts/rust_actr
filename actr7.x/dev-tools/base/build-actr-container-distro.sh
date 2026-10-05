

actr_version=actr$1

cd $actr_version

cd $actr_version

r_ver=`svnversion`

cd ..

cd $r_ver

cd $actr_version


rm -f "environment/Start Environment.exe"
rm -f "environment/start-environment-osx"
rm -f "environment/start-environment-osx-arm64"
rm -f "environment/start-environment-Linux"

rm -f examples/connections/nodejs/html-environment-macos
rm -f examples/connections/nodejs/html-environment-macos-arm64
rm -f examples/connections/nodejs/html-environment-linux
rm -f examples/connections/nodejs/html-environment-win.exe

cd ..

date=`date +%F`

sed "s/\*actr-repository-number\* nil/*actr-repository-number* \"$r_ver\"/" < $actr_version/framework/version-string.lisp >$actr_version/framework/new_version-string.lisp

mv $actr_version/framework/new_version-string.lisp $actr_version/framework/version-string.lisp

sed "s/\*actr-release-tag\* \"\(.*\)\"/*actr-release-tag* \"$date\"/" < $actr_version/framework/version-string.lisp >$actr_version/framework/new_version-string.lisp

mv $actr_version/framework/new_version-string.lisp $actr_version/framework/version-string.lisp

sed "s/\*actr-repository-number\* \"\(.*\)\"/*actr-repository-number* \"\1\.c\"/" < $actr_version/framework/version-string.lisp >$actr_version/framework/new_version-string.lisp

mv $actr_version/framework/new_version-string.lisp $actr_version/framework/version-string.lisp


zip -r distro/actr7.container.zip $actr_version

if [ "$2" == "send" ]; then
  scp distro/actr7.container.zip db30@act-r.psy.cmu.edu:~/www/$actr_version/next
  cd ..
  cd ..
  scp container_readme.txt db30@act-r.psy.cmu.edu:~/www/$actr_version/next

fi
