# ACT-R Rust release

This bundle contains the Rust port of the ACT-R 7 single-threaded kernel:

- `actr` (`actr.exe` on Windows) - the command-line engine and RPC server.
- `wheels/` - the in-process Python module (`import rust_actr`).
- `tutorial/` - the Python ports of the ACT-R tutorial models and tasks.
- `examples/lunar_lander.py` - a small playable example of the Python module API.
- `clients/actr.py` - the stock Python RPC client for `actr server`.
- `LICENSES/` - LGPL, CMU dictionary, and third-party notices.

Notebook versions of the tutorial are not included in this release. The
scripts in `tutorial/` are self-contained and run directly.

## Requirements

- Python 3.9 or newer for the Python module.
- Windows 10+, macOS 11+, or a Linux system with glibc 2.35 or newer for the
  bundled Linux binary.
- The bundled Linux wheel is built for manylinux_2_17.

The binaries are not code-signed. macOS may quarantine the `actr` binary; if
needed, remove the quarantine flag with:

```sh
xattr -d com.apple.quarantine actr
```

Windows SmartScreen may warn about the unsigned `actr.exe`.

## Install the Python module

From this directory:

```sh
python -m pip install wheels/rust_actr-*.whl
```

On Windows, use `py -3 -m pip install wheels\rust_actr-*.whl` if `python` is not
on `PATH`.

## Run the command-line engine

```sh
actr --help
actr run model.lisp
actr server
actr env
```

`actr run` takes a filesystem path to a single model or task file. `actr server`
starts the RPC dispatcher and writes `~/act-r-port-num.txt` for clients such as
`clients/actr.py`. `actr env` opens the native Environment GUI; use
`actr server --env` to run the server and GUI together.

## Run the tutorial

Install the wheel first, then run any tutorial script from anywhere:

```sh
python tutorial/unit1/count.py
python tutorial/unit2/demo2.py
python tutorial/unit5/grouped.py
python tutorial/unit5/onehit.py hands 2
```

The tutorial scripts build their models in Python and do not read the original
`.lisp` sources.

## Use `actr server` from Python

`clients/actr.py` is the same RPC client used by the upstream ACT-R tutorial. It
discovers the server through `~/act-r-address.txt` and `~/act-r-port-num.txt`:

```python
import sys
sys.path.insert(0, "clients")
import actr

actr.connect()
```

The in-process `rust_actr` module does not use the RPC server.

## Licenses

ACT-R is distributed under the GNU Lesser General Public License; see
`LICENSES/LGPL.txt`. The syllable data is derived from the CMU Pronouncing
Dictionary; see `LICENSES/CMUDICT-LICENSE.txt`. Third-party notices are in
`LICENSES/THIRD-PARTY.txt`.
