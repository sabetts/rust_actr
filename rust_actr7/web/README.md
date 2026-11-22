# ACT-R in the browser

A static browser page that runs the Rust ACT-R engine as WebAssembly. It loads
canonical tutorial model source and drives Units 1-5 through a browser AGI
runtime.

## Build + run

From the repository root:

```sh
python rust_actr7/web/build.py
cd rust_actr7/web
python -m http.server 8000
# open http://localhost:8000/
```

`build.py` stages the model files and manifest under `web/models/`, builds the
wasm, and copies it next to `index.html`:

```text
cargo build --manifest-path rust_actr7/Cargo.toml --release --target wasm32-unknown-unknown
```

It needs a Rust toolchain with the `wasm32-unknown-unknown` target:

```sh
rustup target add wasm32-unknown-unknown
```

No `wasm-pack` is needed. To stage only the model menu:

```sh
python rust_actr7/web/build.py --models-only
```

The page loads `./actr_engine.wasm` and the generated `./models/` files, so
serve the `web/` folder over HTTP. Opening `index.html` as `file://` fails on
the fetches.

## Models

The menu is generated from `web/models/manifest.json`.

### Unit 1

- `actr7.x/tutorial/unit1/count.lisp` - Count
- `actr7.x/tutorial/unit1/addition.lisp` - Addition
- `actr7.x/tutorial/unit1/semantic.lisp` - Semantic
- `actr7.x/tutorial/unit1/tutor-model.lisp` - Tutor Model (starter)
- `actr7.x/tutorial/unit1/tutor-model-solution.lisp` - Tutor Model (solution)
- `actr7.x/tutorial/unit1/broken-addition.lisp` - Broken Addition (intentionally broken)

### Unit 2

- `actr7.x/tutorial/unit2/demo2-model.lisp` - Demo2
- `actr7.x/tutorial/unit2/unit2-assignment-model.lisp` - Unit 2 Assignment (starter)

### Unit 3

- `actr7.x/tutorial/unit3/sperling-model.lisp` - Sperling
- `actr7.x/tutorial/unit3/perceptual-motor-issues-model.lisp` - Perceptual-Motor Issues
- `actr7.x/tutorial/unit3/subitize-model.lisp` - Subitize (starter)

### Unit 5

- `actr7.x/tutorial/unit5/fan-model.lisp` - Fan Effect
- `actr7.x/tutorial/unit5/fan-no-pm-model.lisp` - Fan Effect (no PM)
- `actr7.x/tutorial/unit5/grouped-model.lisp` - Grouped Recall
- `actr7.x/tutorial/unit5/siegler-model.lisp` - Siegler Addition
- `actr7.x/tutorial/unit5/1hit-blackjack-model.lisp` - 1-Hit Blackjack (starter)
- `actr7.x/tutorial/unit5/declarative-issues.lisp` - Declarative Issues

The canonical model sources stay in `actr7.x/tutorial/`; the `web/models/` copies
are generated and ignored by git. Selecting a model replaces the textarea
contents; it does not run automatically. Declarative Issues has no task panel;
running it shows the tutorial trace.

## Task runtime

The wasm engine exposes the Rust RPC session's AGI command layer and
visible-window events. The browser side mirrors the in-process Python interface
in `rust_actr7/python`:

- `src/web_session.rs` / `src/wasm.rs` - session bridge, JSON commands, stepped
  run slices, fired commands, trace lines, and visible-window events.
- `web/js/actr.js` - JS ACT-R facade: commands, monitors, windows, devices,
  stepped model runs, ACT-R RNG helpers, and timing/scheduling helpers.
- `web/js/agi-window.js` - HTML visible-window renderer for AGI text, buttons,
  lines, images, attention, and cursor events.
- `web/js/tasks/unit2.js`, `sperling.js`, `pm-issues.js`, `subitize.js`,
  `fan.js`, `grouped.js`, `siegler.js`, `onehit.js` - JS ports of the tutorial
  experiment code.
- `web/js/tasks/index.js` - task registry with controls and default run times.
- `web/js/main.js` - model menu, editor, task controls, reports, and run wiring.

Task Lisp/Python is translated to JS; it is not evaluated in wasm. The task
code still calls the same AGI/RPC commands as the Lisp/Python tasks, including
`open-exp-window`, `add-text-to-exp-window`, `install-device`, `run`,
`add-act-r-command`, and `monitor-command`. Randomized setup uses the engine's
`permute-list` and `act-r-random`, so seeded behavior matches ACT-R. Tasks may
also supply a small model-support `defun` before the model source; 1-Hit
Blackjack uses this for its numeric `:sim-hook`.

Runs with commands or monitors are stepped one engine event at a time. This
lets scheduled task callbacks run mid-run and lets the page repaint when a
window changes, as in PM Issues and Sperling. Tasks that only need feedback
between phases, such as blackjack, use full-time model runs.

## Task controls

Task descriptors can expose controls in the task panel. The current controls
are:

- Sperling: number of blocks
- Subitize: all 1-10 items, or a single item count
- PM Issues: random, next, or previous prompt
- Fan / Fan no PM: both retrieval terms, person only, or location only
- Siegler: number of sets (10 by default)
- 1-Hit Blackjack: learn or play mode, games, hands, and human decision time

Controls initialize to task defaults and are disabled while a task runs.
Switching tasks restores the values last used for that task.

## Notes

- Demo2 displays the canonical `V` because its model fixes the seed.
- Sperling runs one block of four tone-onset trials by default. Tones are
  delivered to the ACT-R audio module but are not played through browser audio.
- The Unit 2 Assignment, Subitize, and 1-Hit Blackjack models are intentionally
  incomplete starters, so their results are not tutorial solutions.
- The Perceptual-Motor Issues task is presented as in the tutorial.
- Fan reports response times and correctness for the target and foil probes,
  with correlation and mean deviation against the tutorial data.
- Grouped Recall uses the model's fixed seed, so it reports the expected
  `1 2 3 4 6 5 7 8` sequence with the commission and omission errors.
- Siegler presents digit sounds to the engine and reports a response matrix
  against the child data. Tones are not played through browser audio.
- 1-Hit Blackjack can run automated learning blocks or a human-vs-model game.
  The play mode supports Hit/Stay buttons and the `h`/`s` keys, with a Stay
  fallback on timeout. The default deck and opponent are the tutorial defaults.
- The engine is deterministic given the seed, so equal inputs give equal traces.

## Limits

- Task Lisp/Python is translated to JS; arbitrary Lisp task evaluation is not
  supported. A task can prepend a small support `defun` for a model hook.
- Human interaction is implemented for 1-Hit Blackjack only; other tasks run
  models only.
- The AGI surface covers the item and device features needed by the current
  tutorial units; more behavior can be added as later units need it.
- Runs happen on the browser main thread. Stepped runs yield for repaints, but
  a pathological model can still block the tab.
- One self-contained model file at a time. Models that need an external
  task/setup file are not supported yet.
- 1-Hit Blackjack uses the default tutorial deck and opponent. Alternate decks
  and the stacked `game1` scenario are not exposed in the UI.
- The Tutor starter model is intentionally incomplete, Broken Addition is
  intentionally broken, and the 1-Hit Blackjack starter learns poorly.
