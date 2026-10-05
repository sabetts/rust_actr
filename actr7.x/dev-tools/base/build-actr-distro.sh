
actr_version=actr$1

mkdir $actr_version

cd $actr_version

svn co svn://act-r.psy.cmu.edu/$actr_version

cd $actr_version

r_ver=`svnversion`

cd ..
mkdir $r_ver

cd $r_ver

cp -r ../$actr_version .

cd $actr_version

rm -rf .svn


rm -rf dev-tools
rm -f examples/*-solutions.zip
rm -rf docs/notes
rm -f docs/template.lisp

find . -name *.odt | while read FILE
do

doc_dir=`dirname $FILE`

 echo $FILE

 libreoffice --headless $FILE "macro:///Standard.Module1.updateall"

 libreoffice --headless --convert-to pdf --outdir $doc_dir $FILE
 rm -f $FILE

done

rm -rf test-models

cd ..
mkdir distro

cp -r $actr_version/tutorial "ACT-R Tutorial"

zip -r distro/units.zip "ACT-R Tutorial"


date=`date +%F`

sed "s/\*actr-repository-number\* nil/*actr-repository-number* \"$r_ver\"/" < $actr_version/framework/version-string.lisp >$actr_version/framework/new_version-string.lisp

mv $actr_version/framework/new_version-string.lisp $actr_version/framework/version-string.lisp

sed "s/\*actr-release-tag\* \"\(.*\)\"/*actr-release-tag* \"$date\"/" < $actr_version/framework/version-string.lisp >$actr_version/framework/new_version-string.lisp

mv $actr_version/framework/new_version-string.lisp $actr_version/framework/version-string.lisp


zip -r distro/$actr_version.zip $actr_version

cp $actr_version/docs/QuickStart.txt distro/
cp $actr_version/docs/AGI.pdf distro/
cp $actr_version/docs/EnvironmentManual.pdf distro/
cp $actr_version/docs/reference-manual.pdf distro/
cp $actr_version/docs/remote.pdf distro/

if [ "$2" == "send" ]; then
  scp distro/* db30@act-r.psy.cmu.edu:~/www/$actr_version/next
fi

mkdir w_stand
mkdir m_stand
mkdir l_stand
cp -r $actr_version w_stand/
cp -r $actr_version m_stand/
cp -r $actr_version l_stand/

cd w_stand

sed "s/\*actr-repository-number\* \"\(.*\)\"/*actr-repository-number* \"\1\.w\"/" < $actr_version/framework/version-string.lisp >$actr_version/framework/new_version-string.lisp

mv $actr_version/framework/new_version-string.lisp $actr_version/framework/version-string.lisp


rm -f "$actr_version/environment/start-environment-osx"
rm -f "$actr_version/environment/start-environment-osx-arm64"
rm -f "$actr_version/environment/start-environment-Linux"

zip -r ../w_$actr_version.zip $actr_version
cd ..


cd m_stand

sed "s/\*actr-repository-number\* \"\(.*\)\"/*actr-repository-number* \"\1\.m\"/" < $actr_version/framework/version-string.lisp >$actr_version/framework/new_version-string.lisp

mv $actr_version/framework/new_version-string.lisp $actr_version/framework/version-string.lisp


rm -f "$actr_version/environment/Start Environment.exe"
rm -f "$actr_version/environment/start-environment-Linux"

zip -r ../m_$actr_version.zip $actr_version
cd ..


cd l_stand

sed "s/\*actr-repository-number\* \"\(.*\)\"/*actr-repository-number* \"\1\.L\"/" < $actr_version/framework/version-string.lisp >$actr_version/framework/new_version-string.lisp

mv $actr_version/framework/new_version-string.lisp $actr_version/framework/version-string.lisp


rm -f "$actr_version/environment/Start Environment.exe"
rm -f "$actr_version/environment/start-environment-osx"
rm -f "$actr_version/environment/start-environment-osx-arm64"

zip -r ../l_$actr_version.zip $actr_version
cd ..


if [ "$3" == "send" ]; then 
  scp ?_$actr_version.zip db30@act-r.psy.cmu.edu:~/www/$actr_version/next
fi

