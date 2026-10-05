scp db30@actr-server.hpc1.cs.cmu.edu:/usr0/home/db30/www/actr$1/next/m_actr$1.zip .

svn co svn://act-r.psy.cmu.edu/actr$1
cd actr$1

rnum="$(svnversion)"
cd ..

mkdir $rnum
cd $rnum

rm -r "ACT-R"
cp -r ../../base/"ACT-R" .

unzip ../m_actr$1.zip

cd ACT-R

cp -r ../actr$1/tutorial .
cp -r ../actr$1/docs .

cp -r ../actr$1/environment .
rm -f environment/*.lisp

cp -r ../actr$1/extras .
cp -r ../actr$1/examples .


mv sbcl_license docs

mv examples/connections/nodejs/act-r.html .
mv examples/connections/nodejs/html-environment-macos apps
mv examples/connections/nodejs/html-environment-macos-arm64 apps
rm -f examples/connections/nodejs/html-environment-linux
rm -f examples/connections/nodejs/html-environment-win.exe

cp ../../readme.txt .
cp ../../run-act-r.command .
cp ../../run-act-r-only.command .
cp ../../run-act-r-from-terminal.sh .
cp ../../run-extra-listener.command .
cp ../../run-html-environment.command .
cp ../../run-act-r-jupyter.command .
cp ../../run_ACT-R.ipynb .

cd ..

$2 --end-runtime-options --eval '(push :standalone *features*)' --eval '(push :actr-recompile *features*)' --eval "(load \"actr$1/load-act-r.lisp\")" --eval '(save-lisp-and-die "act-r-64" :executable t)' --eval '(exit)'

mv act-r-64 ACT-R/apps

zip -r mac_standalone.zip "ACT-R"

cd ..


