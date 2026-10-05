The ACT-R software is available in a container that can be run using Docker 
Desktop (http://www.docker.com) without installing any other software.

The container has the full version of ACT-R running under SBCL and the ACT-R 
Node.js server for the browser based versions of the ACT-R Environment and 
experiment window viewer.

It also includes all of the tutorial materials and an optional Jupyter notebook
server (https://jupyter.org/) along with notebook pages for running all of the
tutorial models and tasks from Python.

To use the container, you need to get it from Docker Hub which can be done like
this once you've installed Docker:

docker pull db30/act-r-container

or from the Docker hub site at:

https://hub.docker.com/r/db30/act-r-container

There are two recommended ways to use the ACT-R in the container.

-------------------------------------------------------------------------------

To run with the Lisp prompt available, the server for the HTML version of the 
ACT-R Environment and experiment window viewer, and the default port for the 
ACT-R remote interface you can run it like this:

Mac and Linux

docker run -i -p 4000:4000 -p 2650:2650 -v ~/act-r-tutorial:/home/actr/actr7.x/tutorial db30/act-r-container

Windows

docker run -i -p 4000:4000 -p 2650:2650 -v %homedrive%%homepath%\act-r-tutorial:/home/actr/actr7.x/tutorial db30/act-r-container

The -p flags are to expose the ports for the HTML Environment interface and  
the ACT-R remote interface respectively.  That allows you to connect to the  
ACT-R Environment from a browser at <http://localhost:4000>, the experiment  
window viewer at <http://localhost:4000/expwindow.html>, and to connect to the
ACT-R remote interface on its default port of 2650.

The -v flag sets up a location on the computer where the tutorial files will be
stored for easier access and so any changes will persist between sessions.  
The value before the : is the path on the local machine to the files.  It can
be anywhere and will create the directory if it does not already exist, but it
is safest to create it in advance because if it gets created by docker it may
not have the proper permissions for the current user.  The calls above create
an act-r-tutorial directory in the user's home directory.  That will not 
overwrite any files already in that directory so you can safely work on those 
files across different runs of the container.


-------------------------------------------------------------------------------

To run the Jupyter server and access ACT-R from notebook pages or the remote 
interface (the Lisp prompt will not be available) you can run it like this:

Mac and Linux

docker run -i -p 4000:4000 -p 2650:2650 -p 8888:8888 -v ~/act-r-tutorial:/home/actr/actr7.x/tutorial db30/act-r-container run-jupyter.sh

Windows

docker run -i -p 4000:4000 -p 2650:2650 -p 8888:8888 -v %homedrive%%homepath%\act-r-tutorial:/home/actr/actr7.x/tutorial db30/act-r-container run-jupyter.sh

The -p flags are to expose the ports for the HTML Environment interface, the 
ACT-R remote interface, and the Jupyter server respectively.  That allows you
to connect to the ACT-R Environment from a browser at <http://localhost:4000>, 
and the experiment window viewer at <http://localhost:4000/expwindow.html>, to 
connect to the ACT-R remote interface on it's default port of 2650, and the
Jupyter notebook server through links which are provided once it finishes 
starting and is ready to use.

The -v flag sets up a location on the computer where the tutorial files will be
stored for easier access and so any changes will persist between sessions.  
The value before the : is the path on the local machine to the files.  It can
be anywhere and will create the directory if it does not already exist, but it
is safest to create it in advance because if it gets created by docker it may
not have the proper permissions for the current user.  The calls above create
an act-r-tutorial directory in the user's home directory.  That will not 
overwrite any files already in that directory so you can safely work on those
files across different runs of the container.

