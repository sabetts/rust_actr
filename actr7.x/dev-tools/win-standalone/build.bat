scp2 db30@actr-server.hpc1.cs.cmu.edu:/usr0/home/db30/www/actr%1/next/w_actr%1.zip .

svn co svn://act-r.psy.cmu.edu/actr%1

cd actr%1

for /f "delims=" %%a in ('svnversion') do @set rnum=%%a
echo this is %rnum%

cd ..

mkdir %rnum%
cd %rnum%

rmdir "ACT-R" /s /q

xcopy ..\..\base\"ACT-R" .\"ACT-R"  /S /E /I

7z x ..\w_actr%1.zip 

cd "ACT-R"
xcopy ..\actr%1\tutorial .\tutorial /S /E /I
xcopy ..\actr%1\docs .\docs /S /E /I

xcopy ..\actr%1\environment .\environment /S /E /I

del .\environment\*.lisp

xcopy ..\actr%1\extras .\extras /S /E /I
xcopy ..\actr%1\examples .\examples /S /E /I

move sbcl_license docs

move examples\connections\nodejs\act-r.html .
move examples\connections\nodejs\html-environment-win.exe apps
del examples\connections\nodejs\html-environment-macos
del examples\connections\nodejs\html-environment-linux


copy ..\..\readme.txt .
copy ..\..\run-act-r.bat .
copy ..\..\run-act-r-only.bat .
copy ..\..\run-extra-listener.bat .
copy ..\..\run-html-environment.bat .
copy ..\..\run-act-r-jupyter.bat .
copy ..\..\run_ACT-R.ipynb .
copy ..\..\run_env.py .

cd ..

%2 --end-runtime-options --eval "(push :standalone *features*)" --eval "(push :actr-recompile *features*)" --load actr%1\load-act-r.lisp --eval "(save-lisp-and-die \"act-r.exe\" :executable t)" --eval "(exit)"

move act-r.exe "ACT-R"\apps

7z a win-standalone.zip "ACT-R"

cd ..
