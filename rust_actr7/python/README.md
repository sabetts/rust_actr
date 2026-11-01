# `rust_actr`: Python interface to the Rust ACT-R engine

Build and run ACT-R models in-process with the Rust engine.

## Install for development

```powershell
python -m pip install maturin
cd rust_actr7/python
maturin build --release
python -m pip install --force-reinstall target/wheels/rust_actr-*.whl
```

The package targets Python 3.9+ through the stable ABI and uses PyO3 0.30.

## Example

```python
import rust_actr as actr

m = actr.Model("count", seed=(100, 0))

m.chunk_type("goal", "state")
start = m.add_chunk("start", "goal", state="start")

m.add_production("""
(p finish
   =goal>
     state start
   ==>
   =goal>
     state done)
""")

m.goal_focus(start)
trace = m.run(1.0)
print(trace)
```

Productions use the same Lisp syntax as `define-model` files, including
`=buffer>` conditions and `==>`.

## Values

Python strings are model symbols, which is what slot conditions usually use:

```python
m.add_chunk("start", "goal", state="start")
```

Use a quoted Lisp string when the model needs one:

```python
m.add_chunk("zero", "number", vocal_rep=actr.String("zero"))
```

Slot names use Python underscores and are rendered as Lisp hyphens:
`next_let` becomes `next-let`.

Supported chunk values are `str`, `int`, `float`, `bool`, `None`, lists/tuples,
and `actr.String`/`actr.LispString`.

## Model lifecycle

- Add chunk types, chunks, productions, and goal focus before the first
  `run()`/`reset()`.
- Definitions are frozen once the engine is built.
- `run(seconds)` returns only the newly emitted trace.
- `model.trace` returns all trace emitted so far.
- `take_trace()` returns and clears the trace history.
- `to_lisp()` returns the generated `(define-model ...)` source.

## Current limits

The interface covers model construction, runtime buffer/DM commands, windows
and devices, command monitors, mid-run task callbacks, Python similarity hooks,
and `mod-chunk`/`copy-chunk`. Arbitrary Lisp evaluation and multiple-model
files are still out of scope.

## Tutorial models

`rust_actr.tutorial` contains Python ports of the model files under
`actr7.x/tutorial`. They are generated from the vendored Lisp sources by
`tools/port_tutorial_models.py` and can be edited after generation:

```powershell
python tools/port_tutorial_models.py
python -m rust_actr.tutorial.run --list-models
```

`rust_actr.actr` is a module-level façade matching the tutorial client. It
binds `load_act_r_model` to the Python registry, so the vendored task scripts
run with their imports unchanged:

```python
from rust_actr.tutorial import runner

runner.run_task("demo2", "experiment")
runner.run_task("fan_no_pm", "experiment")
```

The runner needs an ACT-R checkout for the task scripts. Set `ACTR_DIR` or run
from the repository root:

```powershell
$env:ACTR_DIR = "C:\path\to\actr7.x"
python -m rust_actr.tutorial.run demo2 experiment
```

Port status:

- Working end-to-end: `count`, `addition`, `semantic`, `demo2`, `fan-no-pm`,
  `grouped`, `onehit` (hands/blocks/learning), `zbrodoff` (set/block/experiment),
  and the dynamic models that do not need task-side hooks.
- Python `:sim-hook` functions are wired for `1hit`, `bst-ppm`, and
  `categorize`. Python `imaginal-action` callbacks are also bridged through a
  host-yield step; `bst-ppm` still stops earlier in its live-window/mouse path.
- Starting models (no/partial productions by design): unit2 assignment,
  `subitize`, `choice`, `categorize`.
- `unit1/broken-addition.lisp` is intentionally malformed (the unit uses it
  to demonstrate load warnings) and is not ported.
- Known gaps: the full `bst-ppm` live task traversal, and
  `new-digit-sound` / `new-word-sound` (`siegler`).

## Tests

```powershell
python -m unittest discover -s tests -v
```
