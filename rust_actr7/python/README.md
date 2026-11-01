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

Supported chunk values are `str`, `int`, `float`, `bool`, `None`, and
`actr.String`/`actr.LispString`.

## Model lifecycle

- Add chunk types, chunks, productions, and goal focus before the first
  `run()`/`reset()`.
- Definitions are frozen once the engine is built.
- `run(seconds)` returns only the newly emitted trace.
- `model.trace` returns all trace emitted so far.
- `take_trace()` returns and clears the trace history.
- `to_lisp()` returns the generated `(define-model ...)` source.

## Current limits

The first release covers model construction, goal focus, running, resetting,
and trace access. Devices, windows, task callbacks, arbitrary Lisp evaluation,
runtime buffer mutation, and multiple models are not exposed yet.

## Tests

```powershell
python -m unittest discover -s tests -v
```
