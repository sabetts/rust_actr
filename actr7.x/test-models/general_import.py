import importlib
import os
import sys


def import_path(fullpath,reload=False):
    """ 
    Import a file with full path specification. Allows one to
    import from anywhere, something __import__ does not do. 
    """
    path, filename = os.path.split(fullpath)
    filename, ext = os.path.splitext(filename)
    sys.path.insert(0, path)
    module = __import__(filename)
    if reload:
        importlib.reload(module)
    del sys.path[0]
    return module
