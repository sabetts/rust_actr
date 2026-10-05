cd actr%1
for /f "delims=" %%a in ('svnversion') do @set rnum=%%a
echo this is %rnum%
cd ..

cd %rnum%

scp2 win-standalone.zip db30@actr-server.hpc1.cs.cmu.edu:/usr0/home/db30/www/actr%1/next

cd ..
