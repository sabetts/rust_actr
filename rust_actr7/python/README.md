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

## Modules

Rust and Python can both define cognitive modules. A Python module subclasses
`actr.Module`; the hooks mirror Lisp's `define-module` and are optional except
`name` (and `query`/`params` when the module has buffers/parameters).

```python
import rust_actr as actr


class Game(actr.Module):
    version = "0.1"
    documentation = "game bridge"

    def name(self):
        return "game"

    def buffers(self):
        return [
            actr.BufferSpec("game-state", request_params=["look"]),
            actr.BufferSpec("game-action", request_params=["move"]),
        ]

    def chunk_types(self):
        return [
            actr.ChunkTypeSpec("game-state", slots=["mode", "x", "y"]),
            actr.ChunkTypeSpec("game-action", slots=["cmd"]),
        ]

    def parameters(self):
        return [actr.ParamSpec(":game-action-time", default_value=0.05)]

    def request(self, ctx, buffer, spec):
        ctx.set_busy(buffer, True)
        ctx.schedule_named(50, "done", [spec.slots.get("CMD", "wait")])

    def on_event(self, ctx, event):
        if event.name == "done":
            ctx.set_busy("GAME-ACTION", False)
            ctx.emit("action-complete")

    def query(self, ctx, buffer, name, value):
        # Return True/False for queries you handle, or None to fall back.
        return None

    def params(self, ctx, name, value=actr.UNSET):
        return None


m = actr.Model("model", modules=[Game])
```

Slot and query names are uppercase Lisp symbols (`"CMD"`, `"STATE"`, `"FREE"`).
Use `ctx.schedule_named(...)`/`on_event` for delayed module work; it is an
engine extension, not a `define-module` keyword.

## Lunar lander example

`examples/lunar_lander.py` is a single-file playable demo of the module API.
It has rough terrain, a flat landing pad, three combinable thrusters, and a
small ACT-R model that uses a Python module to observe and control the lander.

```powershell
# Play it yourself (tkinter)
python examples/lunar_lander.py --mode human

# Let the ACT-R model play, headless
python examples/lunar_lander.py --mode model --seed 1

# Watch the ACT-R model play in the game window
python examples/lunar_lander.py --mode model --seed 1 --gui
```

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
`next_let` becomes `next-let`. Parameter names also omit the Lisp colon:
`m.sgp("trace_detail", "high")` renders `(sgp :trace-detail high)`;
`":trace-detail"` is still accepted. Use
`declare_buffer_usage(..., all_slots=True)` for ACT-R's `:all`.

Supported chunk values are `str`, `int`, `float`, `bool`, `None`, lists/tuples,
and `actr.String`/`actr.LispString`.

Speech timing uses `:syllable-rate` and `:char-per-syllable`; the syllable-count
extension adds `:use-syllable-table`, `:split-vocal-strings`,
`register_syllable_count`, and `get_syllable_count`. `new_word_sound` schedules
a `WORD` sound using the same articulation-time calculation.

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
and devices, command monitors, mid-run task callbacks, Python similarity/PPM
hooks, `mod-chunk`/`copy-chunk`, word/tone/digit sounds, the speech articulation
and syllable-count commands, and Python-defined modules. Runtime model loading
uses filesystem paths only; arbitrary Lisp evaluation and multiple-model files
are still out of scope.

Custom modules support buffers, chunk types, requests and modification
requests, queries, owned parameters, lifecycle hooks, and named scheduled
events. Multi-buffer `search`/`offset` and production-time `warning` callbacks
are part of the Python surface but are not invoked until the engine implements
those features.

## Tutorial scripts

`tutorial/` contains self-contained Python ports of the models under
`actr7.x/tutorial`: model-only scripts build `m = actr.Model(...)` and run
top to bottom, while task scripts keep a `build(seed)` function plus their
experiment code. Model symbols are lowercase and productions keep the
tutorial's multi-line layout. Run them from `rust_actr7/python`:

```powershell
python tutorial/unit1/count.py
python tutorial/unit2/demo2.py
python tutorial/unit5/grouped.py
python tutorial/unit5/onehit.py hands 2
```

Model-only scripts print the 10-second trace (the same shape as
`actr run path/to/model.lisp`). Task scripts call a per-file default entry
point; pass a function name and arguments to override it. Arguments are
parsed as JSON when possible.

Regenerate the tree from the vendored sources with:

```powershell
python tools/port_tutorial.py
```

The generated modules are checked in and can be edited after generation.

The tutorial tree is a checkout-only learning resource; it is not installed in
the wheel. The scripts build their models from in-memory source, so they do not
need `ACTR_DIR` or an ACT-R checkout.

The Rust runtime, CLI, server, and GUI accept filesystem paths only.
`ACT-R:` logical names are a fixture/regeneration-tooling concept.

`rust_actr.actr` is the module-level façade the scripts use. You can also bind
a model yourself:

```python
import rust_actr.actr as actr
from tutorial.unit1 import count

actr.use(count.m)
actr.run(10)
print(count.m.trace)
```

## Tutorial notebooks

The tutorial also has Rust-port notebooks next to each unit's scripts:

```text
tutorial/tutorial.ipynb
tutorial/unit1/unit1.ipynb
...
tutorial/unit8/unit8.ipynb
```

Launch Jupyter from `rust_actr7/python` and open `tutorial/tutorial.ipynb`.
The setup cell puts the tutorial tree on `sys.path`, imports `rust_actr.actr`,
and the model cells import the ported Python modules instead of loading Lisp
files. Edit the module next to the notebook and call `actr.reload()` to pick
up changes.

The notebooks keep the original tutorial text and cell order. Cells that need
a human in front of an Experiment window, old `<name>` templates, and heavy
experiment runs are tagged `rust-actr-skip`. Unit 1's intentionally malformed
`broken-addition` section is documented but not ported.

Run every notebook headlessly (skipping tagged cells) with:

```powershell
python tools/run_notebook.py --all
```

Regenerate the notebooks from the vendored Lisp/Python tutorial with:

```powershell
python tools/port_notebooks.py
```

Port status:

- Every checked-in model/task script and tutorial notebook for units 1-8 runs
  through the Python interface. `tests/test_tutorial_units.py` and
  `tests/test_notebooks.py` are the per-unit smoke matrices.
- Working end-to-end: `count`, `addition`, `semantic`, `tutor-model`, `demo2`,
  unit2 assignment, `sperling`, `subitize` (starting model), `pm-issues`,
  `paired`, `zbrodoff` (set/block/experiment), `fan`, `fan-no-pm`, `grouped`,
  `onehit` (hands/learning), `siegler`, `bst` (test), `choice` (starting
  model), `paired-learning`, `past-tense`, `pcomp-issues`, `ul-issues`,
  `bst-ppm`, `categorize` (starting model), and the dynamic models that do not
  need task-side hooks.
- Python `:sim-hook` and `:ppm-hook` callbacks are supported. Register them
  with `actr.add_command` (the facade auto-installs the matching hook) or call
  `model.set_sim_hook` / `model.set_ppm_hook` directly. `:ppm-hook` callables
  receive `(production, [(buffer, slot, expected, actual, similarity), ...])`.
  Python `imaginal-action` callbacks are bridged through a host-yield step.
- The Rust engine resolves model-body `call-act-r-command` forms through Lisp
  defuns only. `categorize` therefore adds its example chunks from the task
  after the model loads. `categorize.experiment` also supports the tutorial's
  offset and alternate feature slot names; changing the slots rebuilds the
  model and regenerates the examples.
- `tutorial/unit8/categorize.py` is a hand-maintained Rust-port override
  (`tools/task_overrides/categorize.py`); `port_tutorial.py` copies it instead
  of regenerating it from the upstream task.
- Starting models (no/partial productions by design): unit2 assignment,
  `subitize`, `choice`, `categorize`.
- `unit1/broken-addition.lisp` is intentionally malformed (the unit uses it
  to demonstrate load warnings) and is not ported.
- Tutorial task modules bind their functions to their own model, so several
  modules can be imported in one process. Full `.ref` parity for every
  experiment summary is still a test-suite expansion, not a missing runtime
  path. Protected solution models are not available upstream; see
  `planning/PARITY-AUDIT.md`.

## Tests

```powershell
python -m unittest discover -s tests -v
```
