
To run the ACT-R Standalone, extract the "ACT-R" folder from the zip archive and
place it anywhere on your machine.  

If you are using macOS 10.14 or newer, then before running the software you will
need to verify that the terminal application has access to the files.  To do
that you need to open System Preferences and then Security & Privacy.  In the
Privacy section, pick "Full Disk Access" from the items on the left.  If the 
terminal application is not listed with a checkmark on the right then you will 
have to add it.  It should be located in the Applications/utilities folder.

To install the software you need to run the "one-time-setup.command" script by 
double clicking on it or running it from the terminal with the ACT-R directory 
as the current directory.  That will add all of the necessary files and folders 
to that folder, and it should also clear the quarantine flags so that the 
applications will be allowed to run without having to explicitly allow them.

If you get a warning that the file is damaged or cannot be opened then you will
need to Control-click the file and pick Open to run it, and that will require
an admin password.  Note, for macOS Sequoia that still won't allow you to open
the file, and you will need to click the done option and then open the System
Settings, select the Privacy & Security settings, and then scroll down to 
the Security section where you should find a warning about running the script
and a button that says "Open Anyway" which you will need to then press to run
the script.

Once that is done, you just need to double click the "run-act-r.command" script
to start ACT-R and the ACT-R environment.  That script must be located in the
same directory as the rest of the ACT-R files, but you may create an alias to
it and put that elsewhere if that is more convenient.  There are also 
additional scripts which can be used for starting pieces of the software
individually, and those are described below.

Alternatively, if you have Jupyter installed you can run the run_ACT-R.ipynb
notebook.  That has cells to run the ACT-R software and the ACT-R Environment,
and it also contains a link to the tutorial notebooks which can be used to run
all of the tutorial models and tasks.

If when you run that you get a warning that the file is damaged or cannot be 
opened (for that file or one of the applications) then you will have to 
explicitly allow it to run and additional instructions for that are also found
below.

Running the run-act-r script will open a terminal window titled "ACT-R" and
start the ACT-R Environment "Control Panel".  If you get a dialog window which
says "Error occurred trying to connect to ACT-R" at the top, press the No 
button to have it try again.  If that works, then to prevent that from 
happening every time you start ACT-R, you will need to press the "Options" 
button at the bottom of the "Control Panel" window, check the box next to "use
localhost instead of 127.0.0.1", and then press the save button.  If the No 
button also does not work, then you may need to adjust the firewall settings to
allow the connection.

To close the ACT-R Standalone you should close the "Control Panel" first and
then close the "ACT-R" window.

See the tutorial, reference manual, and environment manual that are found in
the docs folder for more details.

The ACT-R application is built using Steel Bank Common Lisp (SBCL). SBCL is
free software available from <http://sbcl.org>.  It is mostly in the public 
domain, but with some subsystems under BSD-style licenses which allow 
modification and reuse as long as credit is given.  More information can be
found in the files in the sbcl_license folder in the docs folder.  
-------------------------------------------------------------------------------

Other scripts:

run-act-r-only.command

This script will run only the ACT-R terminal application without starting the
Environment GUI.

run-extra-listener.command

This script can be run to open another terminal running Lisp which connects to
the running ACT-R.  That terminal provides functions/macros to call the ACT-R
commands which are described in the tutorial and may be useful for inspecting
or debugging a model.  By default that terminal will not display the ACT-R
model trace but it does display the other traces.  If the model trace is 
desired in that terminal, the include-model-trace function can be called to add
it.  Any number of extra listeners may be run at the same time.

run-html-environment.command

This script can be run to start an application which allows one to use an
alternate version of the ACT-R Environment that is implemented in javascript
and works from a browser (it can be run instead of or in addition to the
default Tcl/Tk based Environment).  After you start the application you
should then open a browser and open the act-r.html file which is found in
the ACT-R directory.  It will show two links.  One goes to the Environment
tools and the other opens a viewer for the experiment windows created by the
ACT-R AGI tools.

envstarter.py

It is possible to run the standard ACT-R Environment from source code using
Python 3 if there is a problem with the application or you prefer not to use
it.  To do so, run the envstarter.py script in the ACT-R/environment directory
using either "python envstarter.py" or "python3 envstarter.py" depending on
the name of the application for Python 3 on your machine.

-------------------------------------------------------------------------------
Connecting other software to the ACT-R interface

By default the standalone uses the localhost address for the ACT-R interface,
but if you need to make external connections, delete the force-local.lisp file 
from the patches directory before starting the software.

-------------------------------------------------------------------------------

If you have any questions or problems with this please let me know.

Dan (db30@andrew.cmu.edu)
