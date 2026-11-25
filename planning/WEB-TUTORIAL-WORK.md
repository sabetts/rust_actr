# Web Tutorial Work

Last updated: 2026-11-25
Branch: `main`
Latest relevant commit: `2f0ddf0`

This document tracks the browser tutorial under `rust_actr7/web`: what is
implemented, what remains, how it is verified, and the decisions made while
porting tasks and human modes.

## 1. Goal

Keep the static browser page aligned with the ACT-R Units 1-8 tutorial while
providing a usable human-participant mode for tasks where that makes sense.

The page runs the Rust engine as WebAssembly. Task Lisp/Python is ported to
JavaScript and drives the same AGI/RPC commands as the tutorial tasks.

## 2. Current status

All Units 1-8 tutorial models and task ports are present. The model browser,
guides, source editor, trace view, model runs, and browser tests are working.

Human mode is implemented for:

- Unit 2 Demo2
- Unit 2 Assignment
- Sperling
- PM Issues
- Subitize
- Fan and Fan no PM
- Grouped Recall
- Alpha-Arithmetic
- Building Sticks and Building Sticks (PPM)
- Choice
- Paired Associate
- Utility Learning
- Production Compilation
- Semantic Network
- 1-Hit Blackjack

Human mode remains to be added for:

- Siegler
- Past Tense
- Categorize

Shared touch-friendly prompts are in place for all human tasks: the new
Grouped Recall, Utility Learning, and Semantic Network modes, plus the older
keyboard-only Unit 2, Sperling, PM Issues, Subitize, Fan, Alpha-Arithmetic, and
Production Compilation tasks. Keyboard shortcuts remain available.

Paired Associate human mode intentionally keeps the canonical fixed-seed
first-block behavior.

## 3. Decisions

- Human-mode runs use wall-clock timing and do not run the model.
- Human stimulus randomization is local to the page, so model-body fixed seeds
  do not make every human trial identical.
- Sperling human mode plays a WebAudio tone and also shows a top/middle/bottom
  row cue.
- Building Sticks human mode offers the two demo problems plus a full
  15-problem experiment.
- Grouped Recall human mode presents `123 / 456 / 789` for five seconds, clears,
  and scores free recall against the correct `1-9` sequence.
- Utility Learning human mode uses fixed trials, A/B buttons, and two seconds of
  feedback.
- Semantic Network human mode compares human answers with the canonical model
  answers.
- Existing touch controls in Choice, Paired Associate, 1-Hit Blackjack, and
  Building Sticks remain task-specific; new shared controls are being added for
  the remaining keyboard-only human tasks.

## 4. Completed checkpoints

- `61a634f web: add human mode to experiment-window tasks`
  - Shared `web/js/tasks/human.js`
  - Human modes for all experiment-window tasks
  - Building Sticks full human experiment
  - Browser coverage for the new human flows
- `69e01bf web: randomize human stimuli independently of model seed`
  - Local human randomization helpers
  - Sperling, Demo2, and PM Issues decoupled from model fixed seeds
- `2f0ddf0 web: add grouped, utility, and semantic human modes`
  - Shared `web/js/tasks/human-controls.js` choice and text prompts
  - Grouped study/free-recall flow
  - Utility fixed-trial A/B flow with two-second feedback
  - Semantic Yes/No/Don't Know flow with canonical model-answer comparison
  - Browser coverage for all three flows

## 5. Next checkpoints

1. Remaining human modes: Siegler, Past Tense, Categorize.
2. Longer-term engine follow-ups:
   - Generic model-body `call-act-r-command` bridge to JS
   - Reset-cleared runtime DM adds
   - External task/setup files
   - 1-Hit Blackjack alternate decks and `game1`
   - Optional Web Worker execution for long runs

## 6. Verification

Run from the repository root:

```powershell
node rust_actr7/web/test/lisp_scan_test.mjs
node rust_actr7/web/test/bridge_test.mjs
node rust_actr7/web/test/unit8_test.mjs
cd rust_actr7/web
npm run test:e2e
```

The browser tests require a built wasm/model tree:

```powershell
python rust_actr7/web/build.py
```

Latest verified result before the next checkpoints: all four test commands pass.

## 7. Progress log

- 2026-11-25: Created this tracker.
- 2026-11-25: Added Grouped Recall, Utility Learning, and Semantic Network
  human modes, plus shared `human-controls.js` prompts. Browser tests cover the
  new flows. Committed as `2f0ddf0`.
- 2026-11-25: Retrofitted shared choice/text prompts into the remaining
  keyboard-only human tasks. Keyboard shortcuts remain available.
