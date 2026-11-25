# Web Tutorial Work

Last updated: 2026-11-25
Branch: `main`
Latest relevant commit: `d74ed83`

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
- Siegler
- Alpha-Arithmetic
- Building Sticks and Building Sticks (PPM)
- Choice
- Paired Associate
- Past Tense
- Utility Learning
- Production Compilation
- Categorize
- Semantic Network
- 1-Hit Blackjack

Shared touch-friendly prompts are in place for all human tasks. Keyboard
shortcuts remain available.

Paired Associate human mode intentionally keeps the canonical fixed-seed
first-block behavior.

## 3. Decisions

- Human-mode runs use wall-clock timing and do not run the model.
- Human stimulus randomization is local to the page, so model-body fixed seeds
  do not make every human trial identical.
- Sperling human mode plays a WebAudio tone to cue the row; no text hint
  reveals which row to report.
- Building Sticks human mode offers the two demo problems plus a full
  15-problem experiment.
- Grouped Recall human mode shuffles the digits `1-9` into three groups of
  three per trial, presents them for five seconds, clears, and scores free
  recall against the studied sequence. It repeats a configurable number of
  trials (6 by default) and reports per-trial plus aggregate recall.
- Utility Learning human mode uses fixed trials, A/B buttons, and two seconds of
  feedback.
- Semantic Network human mode compares human answers with the canonical model
  answers.
- Siegler human mode plays bundled royalty-free digit MP3s, offers a Replay
  sounds button, and falls back to visible digits. The audio files are kept
  under `web/audio/` with their original Pixabay filenames; no zero file is
  needed because Siegler only presents 1, 2, and 3.
- Past Tense human mode presents all 22 verbs by default, accepts typed forms,
  and scores regular/irregular accuracy with English spelling rules.
- Categorize human mode shows the 10 training examples, converts test stimuli
  to size words, and compares Category 1 choices with the tutorial data.

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
- `11baf17 web: add touch-friendly human task controls`
  - Shared choice/text prompts in the remaining keyboard-only human tasks
  - Auto-submit text prompts and page-level keyboard fallback
  - Browser coverage for button and text-input human flows
  - Catalog highlights advertise the new Grouped, Utility, and Semantic human
    modes
- `4970d60 web: add digit audio assets and playback helper`
  - Royalty-free digit MP3s under `web/audio/`
  - Cached WebAudio playback with replay support and visible fallback
- `d74ed83 web: add siegler, past tense, and categorize human modes`
  - Siegler spoken-digit trials with Replay sounds
  - Past Tense typed inflections with regular/irregular scoring
  - Categorize study and 14-item test flow
  - Browser coverage for all three flows

## 5. Next checkpoints

All tutorial task models now have human mode. Remaining work is longer-term
engine and platform follow-up:

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

Latest verified result: all four test commands pass after the Siegler, Past
Tense, Categorize, audio, and touch-control work.

## 7. Progress log

- 2026-11-25: Created this tracker.
- 2026-11-25: Added Grouped Recall, Utility Learning, and Semantic Network
  human modes, plus shared `human-controls.js` prompts. Browser tests cover the
  new flows. Committed as `2f0ddf0`.
- 2026-11-25: Retrofitted shared choice/text prompts into the remaining
  keyboard-only human tasks. Keyboard shortcuts remain available. Committed as
  `11baf17`.
- 2026-11-25: Added bundled digit audio and human modes for Siegler, Past Tense,
  and Categorize. Browser tests cover all three flows. Committed as `4970d60`
  and `d74ed83`.
- 2026-11-25: Reworked the sidebar for a cleaner first load. The page now starts
  with no model selected and every unit collapsed; the selected-model info card
  replaces the welcome card at the top of the sidebar; the model search, its
  heading, the welcome card's Explore button, and the footer seed note are
  removed. Browser coverage updated for the empty state and card placement.
