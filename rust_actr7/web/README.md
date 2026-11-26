# ACT-R 7 Tutorial Models

A static browser page that runs the Rust ACT-R engine as WebAssembly. It loads
canonical tutorial model source and drives Units 1-8 through a browser AGI
runtime. The page is designed as a first-time tour of ACT-R: pick a model, run
it, and read the trace or experiment window.

## Build + run

From the repository root:

```sh
python rust_actr7/web/build.py
cd rust_actr7/web
python -m http.server 8000
# open http://localhost:8000/
```

`build.py` stages the model files and manifest under `web/models/`, stages the
ACT-R head icon under `web/assets/`, builds the wasm, and copies it next to
`index.html`:

```text
cargo build --manifest-path rust_actr7/Cargo.toml --release --target wasm32-unknown-unknown
```

It needs a Rust toolchain with the `wasm32-unknown-unknown` target:

```sh
rustup target add wasm32-unknown-unknown
```

No `wasm-pack` is needed. To stage only the model menu and static assets:

```sh
python rust_actr7/web/build.py --models-only
```

The page loads `./actr_engine.wasm`, the generated `./models/` files, and the
staged `./assets/actr-head.png` icon, so serve the `web/` folder over HTTP.
Opening `index.html` as `file://` fails on the fetches.

Optional bridge smoke tests (Node 18+, after `build.py`):

```sh
node test/lisp_scan_test.mjs
node test/bridge_test.mjs
node test/unit8_test.mjs
```

`lisp_scan_test.mjs` covers the source scanner used for paren matching,
mismatch diagnostics, and best-effort error anchors. It does not need the wasm
build.

Browser smoke test (Node 20+ and Playwright Chromium, after `build.py`):

```sh
cd rust_actr7/web
npm ci
npx playwright install chromium
npm run test:e2e
```

The `.github/workflows/web.yml` workflow builds the wasm, runs the Node and
browser smoke tests on pull requests, and deploys the page to GitHub Pages from
`main`. The Pages staging step copies `index.html`, `assets/`, `audio/`,
`css/`, `guides/`, `js/`, `models/`, and `actr_engine.wasm`, so keep new static assets
in one of those directories and add them to the workflow if you introduce
another top-level folder. Enable
Pages once under Settings -> Pages -> Build and deployment -> Source: GitHub
Actions. The deployed site is the project's Pages URL (for example
`https://<owner>.github.io/<repo>/`); all fetches are relative, so it works
under that subpath.

## Unit guides

The unit list links to HTML conversions of the tutorial ODTs under
`actr7.x/tutorial/unitN/`: the main unit guide, code walkthrough, modeling notes
where present, and the productions summary. Clicking a guide row opens it in the
workspace Guide tab; the arrow beside the row opens the standalone guide in a
new tab. The converted pages, media, and `guides/manifest.json` are committed
under `web/guides/`.

Lisp code blocks are fixed-width and syntax-highlighted with the same token
palette as the model editor. Regenerate them with:

```sh
python rust_actr7/web/convert_guides.py
```

This needs `pandoc` and ImageMagick (`magick`) on `PATH`. ImageMagick converts
the WMF diagrams in Units 1 and 6 to PNG. The guide pages use `web/css/guide.css`
and respect the theme stored by the main page.

## Page UI

The page is a no-build, dependency-free app shell over the same wasm model
runner:

- **Model browser** (`web/js/catalog.js`): Units 1-8 with unit blurbs,
  per-model descriptions, `Task` tags for models with interactive human modes,
  and `Starter`, `Solution`, `Broken`, and `Demo` badges. Units start collapsed
  and no model is selected on first load; the
  selected-model card replaces the welcome card at the top of the sidebar. A
  hidden native `<select id="model-preset">` stays populated for compatibility
  and test selectors.
- **Unit guides**: each expanded unit ends with links to its converted HTML
  guide, code walkthrough, modeling notes, and productions summary. A guide row
  opens in the workspace Guide tab (with a Close button), while its arrow opens
  the standalone page in a new tab. Guide pages use fixed-width, syntax-
  highlighted Lisp code blocks.
- **Run bar**: sticky across the top of the page, with run time, seed pair
  under "Seed & advanced", live status pill, Run, and Cancel.
- **Workspace tabs**: `Task` (task models only), `Model source`, and `Trace`.
  Task models open on the Task tab; trace-only models open on the source and
  switch to the trace when run.
- **Model source**: line-number gutter, lightweight Lisp syntax highlighting,
  and a production/chunk outline that jumps to a form. Unmatched parens are
  marked red and listed in the toolbar; placing the caret just after a paren
  highlights it and its match. Parse errors highlight the source line and show a
  clickable line chip, with task-prelude offsets mapped back to the editor.
  `Reset` restores the loaded source and `Copy` copies it.
- **Trace**: colorized timestamps, modules, events, output, warnings, and
  errors; filter, wrap/follow toggles, copy, and save. The trace line count and
  metadata turn red when the trace contains an error. Traces up to 12,000 lines
  are colorized; larger runs use a plain, line-numbered fallback. The raw trace
  stays in the hidden `#output` textarea for tooling and tests.
- **First-run guidance**: a dismissible welcome card in the sidebar's top slot,
  which the selected model's info card replaces after a model is chosen; a
  "What is ACT-R?" dialog with a short workflow and glossary; and inline help
  for seeds and fixed-timing tasks.
- **Theme and accessibility**: light/dark theme persisted in `localStorage`,
  keyboard tabs, visible focus rings, reduced-motion support, and a sticky run
  bar that keeps the primary action reachable on mobile.
- **Header and favicon**: the ACT-R head image is staged from
  `src/data/actr-head.png` into `assets/` and used for both the browser tab
  icon and the header brand mark.

Keyboard shortcuts: `Ctrl`/`Cmd` + `Enter` runs the model and `Ctrl`/`Cmd` + `F`
filters the trace when the trace tab is open. `Esc` closes dialogs.

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
- `web/js/tasks/human.js` - shared human-mode helpers for mode controls, keyboard
  input, wall-clock waits, hints, and tone playback.
- `web/js/tasks/human-controls.js` - shared button, choice, and text prompts for
  touch-friendly human input.
- `web/js/tasks/audio.js` - cached digit-audio playback for human tasks.
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
needs a Lisp `defun` prelude; it does not dispatch to a JS command. Dispatching
model-body commands to JS is a known follow-up (see the parity audit's
known follow-ups), and it would also need a reset-cleared runtime DM add to
avoid duplicate examples.

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

- Unit 2 Demo2 / Unit 2 Assignment: model or human mode
- Sperling: model or human mode and number of blocks
- Subitize: model or human mode, and all 1-10 items or a single item count
- PM Issues: model or human mode, and random, next, or previous prompt
- Fan / Fan no PM: model or human mode; the retrieval term applies to model runs only
- Grouped Recall: model or human mode; human list mode (randomized practice or canonical) and human trial count (3 by default, practice only)
- Siegler: model or human mode; model sets (10 by default) or human sets (2 by default)
- 1-Hit Blackjack: learn or play mode, games, hands, and human decision time
- Building Sticks / Building Sticks (PPM): report, visible demo, or human mode; report runs, demo problem, and the human problem or full 15-problem experiment
- Choice: model or human mode, model runs, and visible-window toggle
- Paired Associate: model or human mode, pairs, blocks, runs, and model trace
- Alpha-Arithmetic: model or human runner; model trial scope and runs; human trial scope; and single-problem fields
- Past Tense: model or human mode; model trial count and block size, or human verb count
- Utility Learning Issues: model or human mode; model runs use the global run-time field and human runs use a fixed trial count
- Production Compilation Issues: model or human mode; model games and compilation toggle, or human trial count
- Categorize: model or human mode; model runs, offset, and four feature slot names, or human study time
- Semantic Network: model or human mode, and question (`g1`, `g2`, `g3`, or all)

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
- Human-mode runs use wall-clock timing and do not run the model. Keyboard
  input is captured from the page; click the task window or page before typing
  if a form control has focus. Human stimulus randomization is local, so
  model-body fixed seeds do not make every human trial identical. Human tasks
  use the shared button, choice, and text prompts in
  `web/js/tasks/human-controls.js` for touch input while keeping keyboard
  shortcuts.
- Sperling runs one block of four tone-onset trials by default. In model mode,
  tones are delivered to the ACT-R audio module but are not played through
  browser audio. Human mode plays the tone through WebAudio; no text hint
  reveals the cued row. Both modes show block/trial progress.
- The Unit 2 Assignment, Subitize, and 1-Hit Blackjack models are intentionally
  incomplete starters, so their results are not tutorial solutions.
- Subitize shows `Item X/10` progress for the full item set, or `Item 1/1` for a
  single selected count. The ten-item response is labeled `10`; its keyboard
  shortcut is `0`.
- Paired Associate runs the Unit 4 experiment task against the Unit 4, Unit 7,
  or Unit 8 model. The Unit 4 model is traced off by default for full runs;
  enable the task Trace control for short runs. In human mode, type the paired
  digit (0-9) while the word is shown; no click is needed. The visible guide
  labels the recall and study phases.
- Alpha-Arithmetic is the Unit 4 assignment starter: it counts through the
  alphabet and does not retrieve learned additions. Its poor fit is expected.
  Human runs default to a single problem; set, block, and full experiment scopes
  are also available. Human and model runs show trial progress.
- Past Tense is the Unit 7 assignment starter, so it produces no inflections,
  no rewards, and no U-shaped learning. The block table and chart diagnose that
  state rather than presenting a solution. Human mode presents all 22 verbs in
  random order with a `Verb X/Y` prompt, accepts typed past-tense forms, and
  scores regular and irregular accuracy.
- Utility Learning Issues uses its own fixed seed and begins from a
  model-scheduled event, so it is not reset with the page seed. Human mode
  presents A/B choices, gives feedback for two seconds, and uses a fixed trial
  count instead of the global run time.
- Production Compilation Issues runs the canonical starter. The compilation
  toggle applies `:epl`; the tutorial's later utility and reward edits are not
  baked into the model. Human mode runs one game of 150 trials by default and
  accepts a shorter trial count; prompts show trial and block progress.
- The Perceptual-Motor Issues task is presented as in the tutorial.
- Fan reports response times and correctness for the target and foil probes,
  with correlation and mean deviation against the tutorial data. Human mode,
  including Fan no PM, presents one 18-probe pass in a visible sentence window
  with `Probe X/18` progress; model mode shows probe progress as well.
- Grouped Recall uses the model's fixed seed, so it reports the expected
  `1 2 3 4 6 5 7 8` sequence with the commission and omission errors. Human
  mode offers randomized practice (3 trials by default, configurable) or the
  canonical one-trial list `123 / 456 / 789`. Both show three groups of three
  for five seconds and score free recall against the studied sequence. The
  report includes per-trial recall and the aggregate mean and exact-match
  count.
- Siegler presents digit sounds to the engine and reports a response matrix
  against the child data. Model-mode tones are not played through browser
  audio. Human mode plays bundled royalty-free digit MP3s, offers a Replay
  sounds button, falls back to showing the digits if audio is unavailable, and
  shows set/problem progress.
- 1-Hit Blackjack can run automated learning blocks or a human-vs-model game.
  The play mode supports Hit/Stay buttons and the `h`/`s` keys, with a Stay
  fallback on timeout and `Hand X/Y` progress. The default deck and opponent are
  the tutorial defaults.
- Building Sticks runs its model experiment in JS so the browser does not
  depend on the engine's test-only native BST driver. Report mode uses 100 runs
  by default. Human mode offers the two tutorial demo problems or a full
  15-problem experiment with `Problem X/15` progress; the full run reports each
  problem's first choice and the over/under percentages against the tutorial
  data.
- Choice is a starter harness: the empty model produces no responses until
  productions are added. Human mode accepts `h`/`t` keys.
- Building Sticks (PPM) uses the JS `:sim-hook` and imaginal-action callbacks to
  supply `bst-number-sims` and `bst-compute-difference`. Its report defaults to
  10 runs; demo mode uses the tutorial demo problems and human mode offers the
  demo problems or the full 15-problem experiment.
- Simple PPM and Simple Dynamic have no task panel; running them shows the
  tutorial trace.
- Semantic Network runs the tutorial `g1` -> `g2` -> `g2` -> `g3` sequence and
  reports each answer, or runs a single selected question. Human mode asks the
  g1/g2/g3 questions with Yes/No/Don't Know buttons and compares the answers
  with the canonical model answers.
- Paired Associate (Dynamic) uses the shared paired-associate task and enables
  production compilation in the model.
- Categorize is the Unit 8 starter: its model has no productions. The task
  prelude creates the example memories and supplies the Gaussian
  `size-similarities` hook, so the report shows the empty response columns and
  explains that no category responses are expected. It accepts the tutorial's
  offset and alternate feature slot names; invalid or duplicate slots fall
  back to `eh`, `es`, `nl`, `mh` with a report warning, and the example
  memories are regenerated for the chosen slots. Human mode shows the 10
  training examples, converts the 14 test stimuli to size words, and asks
  Category 1/2 for each item.
- The engine is deterministic given the seed, so equal inputs give equal traces.

## Limits

- Task Lisp/Python is translated to JS; arbitrary Lisp task evaluation is not
  supported. Embedder hooks (`:sim-hook`, `:ppm-hook`, imaginal actions) may be
  JS commands; a model-body `call-act-r-command` still needs a Lisp `defun`
  prelude. The generic host-command bridge is a known follow-up; it would also
  need a reset-cleared runtime DM add.
- Human interaction is implemented for every model with a task panel. The
  trace-only models (Unit 1, Declarative Issues, Simple PPM, and Simple
  Dynamic) have no task panel. Categorize human mode uses size words as an
  approximation of the numeric tutorial stimuli.
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
