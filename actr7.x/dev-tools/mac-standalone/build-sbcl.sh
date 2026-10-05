scp db30@actr-server.hpc1.cs.cmu.edu:/usr0/home/db30/www/actr$1/next/mac_standalone.zip .

svn co svn://act-r.psy.cmu.edu/actr$1
cd actr$1

rnum="$(svnversion)"


cd ..

mkdir $rnum
cd $rnum

rm -r actr$1

scp db30@actr-server.hpc1.cs.cmu.edu:/usr0/home/db30/www/actr$1/next/m_actr$1.zip .

unzip m_actr$1.zip

rm m_actr$1.zip

cd actr$1

$2 --end-runtime-options --eval '(push :standalone *features*)' --eval '(push :actr-recompile *features*)' --load load-act-r.lisp --eval '(setf *actr-version-string* (substitute #\s #\m *actr-version-string*))' --eval '(setf (meta-p-version (current-mp)) *actr-version-string*)' --eval '(save-lisp-and-die "act-r-arm64" :executable t)' --eval '(exit)'

cd ..

rm -r "ACT-R"

unzip ../mac_standalone.zip
rm mac_standalone.zip

mv actr$1/act-r-arm64 ACT-R/apps

mv ACT-R .content

mkdir ACT-R

mv .content ACT-R

cp ../one-time-setup.command ACT-R
cp ../readme.txt ACT-R

zip -r mac_standalone.zip "ACT-R"



