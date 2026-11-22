# ACT-R 7 Tutorial Models

A static browser page that runs the Rust ACT-R engine as WebAssembly. It loads
canonical tutorial model source and drives Units 1-8 through a browser AGI
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

Optional bridge smoke tests (Node 18+, after `build.py`):

```sh
node test/bridge_test.mjs
node test/unit8_test.mjs
```

Browser smoke test (Node 20+ and Playwright Chromium, after `build.py`):

```sh
cd rust_actr7/web
npm ci
npx playwright install chromium
npm run test:e2e
```

The `.github/workflows/web.yml` workflow builds the wasm, runs the Node and
browser smoke tests on pull requests, and deploys the page to GitHub Pages from
`main`. Enable it once under Settings -> Pages -> Build and deployment ->
Source: GitHub Actions. The deployed site is the project's Pages URL (for
example `https://<owner>.github.io/<repo>/`); all fetches are relative, so it
works under that subpath.

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

### Unit 4

- `actr7.x/tutorial/unit4/paired-model.lisp` - Paired Associate
- `actr7.x/tutorial/unit4/zbrodoff-model.lisp` - Alpha-Arithmetic (starter)

### Unit 5

- `actr7.x/tutorial/unit5/fan-model.lisp` - Fan Effect
- `actr7.x/tutorial/unit5/fan-no-pm-model.lisp` - Fan Effect (no PM)
- `actr7.x/tutorial/unit5/grouped-model.lisp` - Grouped Recall
- `actr7.x/tutorial/unit5/siegler-model.lisp` - Siegler Addition
- `actr7.x/tutorial/unit5/1hit-blackjack-model.lisp` - 1-Hit Blackjack (starter)
- `actr7.x/tutorial/unit5/declarative-issues.lisp` - Declarative Issues

### Unit 6

- `actr7.x/tutorial/unit6/bst-model.lisp` - Building Sticks
- `actr7.x/tutorial/unit6/choice-model.lisp` - Choice (starter)

### Unit 7

- `actr7.x/tutorial/unit7/paired-learning-model.lisp` - Paired Associate Learning
- `actr7.x/tutorial/unit7/past-tense-model.lisp` - Past Tense (starter)
- `actr7.x/tutorial/unit7/utility-learning-issues-model.lisp` - Utility Learning Issues
- `actr7.x/tutorial/unit7/production-compilation-issues-model.lisp` - Production Compilation Issues (starter)

### Unit 8

- `actr7.x/tutorial/unit8/simple-ppm-model.lisp` - Simple PPM
- `actr7.x/tutorial/unit8/bst-ppm-model.lisp` - Building Sticks (PPM)
- `actr7.x/tutorial/unit8/simple-dynamic-model.lisp` - Simple Dynamic
- `actr7.x/tutorial/unit8/semantic-dynamic-model.lisp` - Semantic Network (Dynamic)
- `actr7.x/tutorial/unit8/paired-dynamic-model.lisp` - Paired Associate (Dynamic)
- `actr7.x/tutorial/unit8/categorize-model.lisp` - Categorize (starter)

Unit 4, Unit 7, and Unit 8 use separate paired-associate models but share the
same JS task implementation. The Unit 7 and Unit 8 models start by clearing the
loaded Unit 4 model and substituting their own productions; the experiment task
is unchanged.

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
  `paired.js`, `zbrodoff.js`, `fan.js`, `grouped.js`, `siegler.js`,
  `onehit.js`, `bst.js`, `choice.js`, `past-tense.js`, `ul-issues.js`,
  `pcomp-issues.js`, `categorize.js`, `semantic-dynamic.js` - JS ports of the
  tutorial experiment code. `bst.js` exports both the Unit 6 task and the
  Unit 8 PPM variant.
- `web/js/tasks/index.js` - task registry with controls and default run times.
- `web/js/main.js` - model menu, editor, task controls, reports, and run wiring.

Task Lisp/Python is translated to JS; it is not evaluated in wasm. The task
code still calls the same AGI/RPC commands as the Lisp/Python tasks, including
`open-exp-window`, `add-text-to-exp-window`, `install-device`, `run`,
`add-act-r-command`, and `monitor-command`. Randomized setup uses the engine's
`permute-list` and `act-r-random`, so seeded behavior matches ACT-R. The wasm
bridge also exposes synchronous `:sim-hook` and `:ppm-hook` callbacks plus the
imaginal-action host request, so a JS command can fill the same role as a
Python hook. The facade installs the callback when the model's hook name
matches the registered command. A task can still prepend a small Lisp `defun`
before the model source; Categorize uses this for `create-example-memories`,
which the model invokes from its body.

### Embedder hooks

The wasm bridge exposes the synchronous value hooks and the imaginal-action
host request:

- `:sim-hook`: register a command named by the model's `:sim-hook` with
  `actr.addCommand(name, (a, b) => number | null)`.
- `:ppm-hook`: register a command named by the model's `:ppm-hook` with
  `actr.addCommand(name, (production, rows) => number | null)`. Each row is
  `[buffer, slot, expected, actual, similarity]`.
- Imaginal actions: a pending `+imaginal-action>` runs the registered command;
  return a chunk name, a flat `[slot, value, ...]` list, or `null`.

`null`, `NaN`, and `false` mean “no value, fall through” to the Lisp hook and
similarity table. Hooks run synchronously inside the engine, so they must not
call back into wasm (no `actr.eval`, `run`, etc.). The web page is single-model:
one active hook per kind per session. A model-body `call-act-r-command` still
needs a Lisp `defun` prelude; it does not dispatch to a JS command.

Runs are cooperative. The JS runner slices every model through the engine's
`run-chunk` API, batches events when no callbacks are needed, and yields to the
browser every few milliseconds. Models with commands or monitors still step
one event at a time so task callbacks run in order. Visible windows repaint
between events; long hidden runs, such as the BST report, remain responsive.

The Run control disables while a run is active and a Cancel button appears.
Cancellation aborts the current model/task run, keeps any partial trace, and
marks the status as `cancelled`. Human waits in the browser tasks also listen
for cancellation.

## Task controls

Task descriptors can expose controls in the task panel. The current controls
are:

- Sperling: number of blocks
- Subitize: all 1-10 items, or a single item count
- PM Issues: random, next, or previous prompt
- Fan / Fan no PM: both retrieval terms, person only, or location only
- Siegler: number of sets (10 by default)
- 1-Hit Blackjack: learn or play mode, games, hands, and human decision time
- Building Sticks / Building Sticks (PPM): report, visible demo, or human mode; report runs and demo problem
- Choice: model or human mode, model runs, and visible-window toggle
- Paired Associate: model or human mode, pairs, blocks, runs, and model trace
- Alpha-Arithmetic: experiment, block, set, or single-problem mode; experiment runs and problem fields
- Past Tense: trial count and block size
- Utility Learning Issues: uses the global run-time field
- Production Compilation Issues: games and compilation toggle
- Categorize: experiment runs
- Semantic Network: question (`g1`, `g2`, `g3`, or all)

Controls initialize to task defaults and are disabled while a task runs.
Switching tasks restores the values last used for that task. Controls can be
hidden when the selected task mode does not use them.

The global run-time input follows the selected task. It is labeled "Trial
time" when the value applies to each trial. For tasks with fixed trial timing,
such as Paired Associate, Alpha-Arithmetic, Past Tense, 1-Hit Blackjack,
Choice, Production Compilation Issues, Categorize, and Semantic Network, the
input is disabled and a help tooltip explains why.

## Notes

- Demo2 displays the canonical `V` because its model fixes the seed.
- Sperling runs one block of four tone-onset trials by default. Tones are
  delivered to the ACT-R audio module but are not played through browser audio.
- The Unit 2 Assignment, Subitize, and 1-Hit Blackjack models are intentionally
  incomplete starters, so their results are not tutorial solutions.
- Paired Associate runs the Unit 4 experiment task against the Unit 4, Unit 7,
  or Unit 8 model. The Unit 4 model is traced off by default for full runs;
  enable the task Trace control for short runs. In human mode, type the paired
  digit (0-9) while the word is shown; no click is needed. The visible guide
  labels the recall and study phases.
- Alpha-Arithmetic is the Unit 4 assignment starter: it counts through the
  alphabet and does not retrieve learned additions. Its poor fit is expected.
- Past Tense is the Unit 7 assignment starter, so it produces no inflections,
  no rewards, and no U-shaped learning. The block table and chart diagnose that
  state rather than presenting a solution.
- Utility Learning Issues uses its own fixed seed and begins from a
  model-scheduled event, so it is not reset with the page seed.
- Production Compilation Issues runs the canonical starter. The compilation
  toggle applies `:epl`; the tutorial's later utility and reward edits are not
  baked into the model.
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
- Building Sticks runs its model experiment in JS so the browser does not
  depend on the engine's test-only native BST driver. Report mode uses 100 runs
  by default; demo and human modes use the tutorial demo problems.
- Choice is a starter harness: the empty model produces no responses until
  productions are added. Human mode accepts `h`/`t` keys.
- Building Sticks (PPM) uses the JS `:sim-hook` and imaginal-action callbacks to
  supply `bst-number-sims` and `bst-compute-difference`. Its report defaults to
  10 runs; demo and human modes use the tutorial demo problems.
- Simple PPM and Simple Dynamic have no task panel; running them shows the
  tutorial trace.
- Semantic Network runs the tutorial `g1` -> `g2` -> `g2` -> `g3` sequence and
  reports each answer, or runs a single selected question.
- Paired Associate (Dynamic) uses the shared paired-associate task and enables
  production compilation in the model.
- Categorize is the Unit 8 starter: its model has no productions. The task
  prelude creates the example memories and supplies the Gaussian
  `size-similarities` hook, so the report shows the empty response columns and
  explains that no category responses are expected. It uses the default feature
  slots (`eh`, `es`, `nl`, `mh`) and zero offset.
- The engine is deterministic given the seed, so equal inputs give equal traces.

## Limits

- Task Lisp/Python is translated to JS; arbitrary Lisp task evaluation is not
  supported. Embedder hooks (`:sim-hook`, `:ppm-hook`, imaginal actions) may be
  JS commands; a model-body `call-act-r-command` still needs a Lisp `defun`
  prelude.
- Human interaction is implemented for 1-Hit Blackjack, Building Sticks
  (including the PPM variant), Choice, and the shared paired-associate task;
  other tasks run models only.
- The AGI surface covers the item and device features needed by the current
  tutorial units; more behavior can be added as later units need it.
- Runs happen on the browser main thread. The runner yields for progress and
  repaints, but an individual long engine event can still block briefly.
  Cancel stops the run at the next chunk boundary.
- One self-contained model file at a time. Models that need an external
  task/setup file are not supported yet.
- 1-Hit Blackjack uses the default tutorial deck and opponent. Alternate decks
  and the stacked `game1` scenario are not exposed in the UI.
- Categorize's canonical model is a starter with no productions. Producing the
  tutorial's categorization fit needs the protected solution model, which is
  not in the repository.
- The Tutor starter model is intentionally incomplete, Broken Addition is
  intentionally broken, and the 1-Hit Blackjack, Choice, Alpha-Arithmetic,
  Past Tense, Production Compilation Issues, and Categorize starters are not
  tutorial solutions.
- Tutorial/regression experiment drivers in the Rust engine are test oracles,
  not browser features. See `planning/TUTORIAL-TEST-DRIVERS.md` for the cleanup
  backlog.
