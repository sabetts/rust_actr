# Simple module that will run the Environment
# using the Tck/Tk built into Python assuming
# this is run in a notebook cell from a notebook
# at the top level of the standalone distribution:
# 
# import run_env
# import multiprocessing
# multiprocessing.Process(target=run_env.run_env).start() 


import tkinter

x = tkinter.Tk()

def run_env():
    x.tk.eval('cd environment')
    x.tk.eval('cd gui')
    x.tk.eval('source pstarter.tcl')
    x.mainloop()
