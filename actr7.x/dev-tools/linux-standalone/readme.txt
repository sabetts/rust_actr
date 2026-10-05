
To run the ACT-R Standalone extract the "ACT-R" folder from the zip archive
and place it anywhere on your machine.  

Then you just need to run the "run-act-r.command" script to start ACT-R and the
ACT-R environment.  That script must be located in the same directory as the
rest of the ACT-R files, but you may create an alias to it and put that
elsewhere if that is more convenient.

Alternatively, if you have Jupyter installed you can run the run_ACT-R.ipynb
notebook.  That has a cell to run the ACT-R software and a link to the 
tutorial notebooks which can be used to run all of the tutorial models and
tasks.

When you run that script it will open a terminal window titled "ACT-R" and 
start the ACT-R Environment "Control Panel".  Note, if you are running the
script from the desktop interface and you don't see the terminal window open,
then you may need to right click it and select "Run in terminal".  If that is
not an option (some Linux desktop interfaces don't allow running scripts from
the GUI) then you will need to open a terminal and run the script directly.

To close the ACT-R Standalone you need to close the "ACT-R" window.  That will
quit the terminal and the Environment will ask to verify that it should quit.

By default the software will use the localhost address for the ACT-R interface,
but if you need to make external connections delete the force-local.lisp file
from the patches directory before starting the software.

The "run-act-r-only.command" script can be run to open only the ACT-R terminal
application without the Environment.

The "run-extra-listener.command" script can be run to open another terminal
running Lisp which connects to the running ACT-R.  That terminal provides
functions/macros to call the ACT-R commands which are described in the tutorial
and may be useful for inspecting or debugging a model.  By default that 
terminal will not display the ACT-R model trace but it does display the other
traces.  If the model trace is desired in that terminal, the function
include-model-trace can be called to add it.  Any number of extra listeners may
be run at the same time.

The "run-html-environment.command" script can be run to start an application
which allows one to use an alternate version of the ACT-R Environment that
is implemented in javascript and works from a browser (it can be run instead
of or in addition to the default Tcl/Tk based Environment).  After you start
the application you should then open a browser and open the act-r.html file
which is found in the ACT-R directory.  It will show two links.  One goes
to the Environment tools and the other opens a viewer for the experiment
windows created by the ACT-R AGI tools.

The "run-act-r-jupyter.command" script should not be run directly.  It should
only be run through the Jupyter interface.

You can also run the standard ACT-R Environment from source code using Python 3
if there is a problem with the application or you prefer not to use it.  To
do so run the envstarter.py script in the ACT-R/environment directory using
either "python envstarter.py" or "python3 envstarter.py" depending on the
name of the application for Python 3 on your machine.


See the tutorial, reference manual, and environment manual for more details.

The ACT-R application is built using Steel Bank Common Lisp (SBCL). SBCL is
free software available from <http://sbcl.org>.  It is mostly in the public
domain, but with some subsystems under BSD-style licenses which allow 
modification and reuse as long as credit is given.  More information can be
found in the files in the sbcl_license folder in the docs folder.

If you have any questions or problems with this please let me know.

Dan (db30@andrew.cmu.edu)
