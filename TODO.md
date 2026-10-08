# TODO — ACT-R 7 test-model coverage

Status legend: `- [ ]` todo · `- [x]` done · `(IN PROGRESS)` picked up.

## Ground rules

- **Oracle**: `actr7.x/test-models/*.ref`. Reproduce it **byte-exact**, even where
  it contradicts current ACT-R source.
- **Keep `cargo test` green.** `rust_actr7/golden/test-models/enabled.txt` is the
  green allowlist; only add a spec there once it matches.
- **Commit as work proceeds**, identity:
  `git -c user.name=sabetts -c user.email=sabetts@localhost commit`.
- **GOTCHAS.md**: add an entry for non-obvious behavior (tied to a source line),
  not a changelog.
- **No command layer.** COMMAND-scope specs and unit-test refs are out of scope
  unless re-triaged in Phase B.
- Device sidecars (`*.device.sexp`) are hand-authored and must not be clobbered;
  spec regeneration is additive (`SPEC_LIST`).

## Recon (2026-10-08)

- 571 `.ref` files (563 top-level); **219 specs**; **58 enabled/green**;
  **2 failing**; **159 ignored**.
- Of the ignored: **156** are `:COMMAND` (classified from stub source tokens in
  `golden/extract-specs.lisp`, which is over-broad). Cross-checking the refs:
  **23 are pure model traces**, **42 near-pure** (≤3 non-trace lines), **91 have
  real command output**.
- The remaining **344 non-timestamped refs** are unit-test/command dumps
  (chunk-type/sgp/production-text) — out of scope.
- Engine is split into `rust_actr7/src/engine/*.rs`; crate is `cargo fmt`-clean.

## How to run

```sh
cd rust_actr7
# probe every runnable spec (pass/fail + first diff)
cargo test --test test_models probe_runnable -- --ignored --nocapture
# ... with the ignored reasons appended
PROBE_IGNORED=1 cargo test --test test_models probe_runnable -- --ignored --nocapture
# dump one spec's actual trace, line-numbered
DUMP_SPEC=<name> cargo test --test test_models dump_spec -- --ignored --nocapture
```

Regenerate specs (additive; never clobbers hand-authored device sidecars):

```sh
# all timestamped refs:
rust_actr7/golden/regen-specs.sh
# explicit subset (one test name per line, no .ref):
SPEC_LIST=/tmp/opencode/spec-list rust_actr7/golden/regen-specs.sh
```

Enable a passing spec by appending its name to
`rust_actr7/golden/test-models/enabled.txt`.

## Definition of done (per task)

- [ ] spec(s) added to `enabled.txt`
- [ ] `cargo test` green
- [ ] committed
- [ ] `GOTCHAS.md` entry if the fix is non-obvious

---

## Phase A — close the frontier (2 known failures)

- [ ] **A1. Sperling vision finst model** → enable
      `tutorial-unit3-sperling-trial-trace`. **(IN PROGRESS)**
  - [ ] Add a vision finst list + `NEW` state on visicon feature creation.
  - [ ] `:attended` semantics: `:attended nil` matches `feat-attended ∈ {NEW, nil}`;
        `:attended new` matches `NEW`; `t`/`requested` excluded
        (`vision.lisp:2269`, `test-attended` `vision.lisp:2006`).
  - [ ] `add-finst` on `encoding_complete`; expire by `:visual-finst-span`
        (`:declarative-finst-span` for DM is already modelled).
  - [ ] Expected effect: 5th find `VISUAL-LOCATION7` → `VISUAL-LOCATION1`, and the
        report phase moves `1.076` → `1.110`.
  - [ ] Enable the spec; commit; GOTCHA.
  - Refs: `engine/perception.rs` (`find_location`, `encoding_complete`,
    `loc_matches`/`loc_rel_matches`, `Feature`), `engine.rs` (`Feature` struct),
    `actr7.x/core-modules/vision.lisp:2006,2269,2462`.

- [ ] **A2. paired-learning utility / selection** → enable `paired-learning`.
  - [ ] First diff: `production1 :utility 0.121` (ref) vs `-0.025` (engine) after
        recreation (`recreating production production1`).
  - [ ] Audit `:u` init vs computed utility, and reward propagation onto
        recreated/compiled productions (`engine/productions.rs`,
        `engine/compilation.rs`, `model.rs` `spp :u`/`:reward`).
  - [ ] Work the remaining ~180 diff hunks (2730 ref lines vs 2808 engine lines).
  - [ ] Enable; commit; GOTCHA.

## Phase B — unlock hidden real-model traces (coverage ROI)

- [ ] **B1. Content-based scope triage.** Decide a spec's scope from the **ref
      content**, not the stub source tokens, so pure/near-pure model traces stop
      being excluded by an over-broad `*command-tokens*` match
      (`golden/extract-specs.lisp` `scope-of`). Keep an explicit out-of-scope
      override for genuine command output. Optionally add a `PROBE_SCOPE` report
      mode to `tests/test_models.rs`.
- [ ] **B2. Regenerate the 23 pure-trace candidates** (additive `SPEC_LIST`),
      recording real steps/seeds. Candidates:
      `hear-self`, `imaginal-action`, `tutorial-unit1-semantic-g2-trace`,
      `tutorial-unit1-semantic-g3-trace`,
      `tutorial-unit2-demo2-find-unattended-letter-actions`,
      `tutorial-unit2-demo2-visual-reencode-actions`,
      `tutorial-unit2code-demo2-manual-actions-trace`,
      `tutorial-unit3-sperling-first-production`,
      `tutorial-unit3-sperling-stuff-visual-location`,
      `tutorial-unit3code-sperling-clear-exp-window-trace`,
      `tutorial-unit3modeling-v3-clear-imaginal`,
      `tutorial-unit3modeling-v4-next-segment-trace`,
      `tutorial-unit3modeling-v4-previous-segment-trace`,
      `tutorial-unit5-count-mod-goal-event`,
      `tutorial-unit5-count-mod-request-goal-event`,
      `tutorial-unit5-imaginal-mod-event`,
      `tutorial-unit5-imaginal-mod-request-event`,
      `tutorial-unit5modeling-v7-p4-run`,
      `tutorial-unit5modeling-v8-p6-run-high-detail`,
      `tutorial-unit5modeling-v9-p6-retrieval`,
      `tutorial-unit5modeling-v9-p8-actions`,
      `tutorial-unit7modeling-ul-issues-v1-second-run`,
      `tutorial-unit8-simple-dynamic-extend-buffer-chunk`.
- [ ] **B3. Probe + enable passers**; file every mismatch as a Phase C task.
- [ ] **B4. Near-pure subset (42).** Reproduce the small non-trace lines
      (`? (run n)` echoes, `...`, `Events in the queue:`) or reclassify them.
      Sample: `tutorial-unit1modeling-broken-addition-v{3,4,5,6,7,9,12}-run*`,
      `tutorial-unit5modeling-v6-p4-run-queue`,
      `tutorial-unit7modeling-pcomp-issues-v9-non-win-composition`.

## Phase C — engine features surfaced by B

- [ ] **C1.** Cluster the new mismatches by subsystem (vision / audio / DM /
      production-compilation / imaginal / temporal / …) and fix one cluster at a
      time; each cluster gets its own spec(s) in `enabled.txt`.

## Phase D — in-scope leftovers

- [ ] **D1.** `unit-1-together-1-mp` — multi-model trace (model column). Needs
      the engine to run several models and emit the model-column trace
      (`golden/test-models/specs/unit-1-together-1-mp.spec`).
- [ ] **D2.** `tutorial_unit2code_initial` — extraction error: the stub loads a
      missing `.lisp`. Fix the extractor or skip it.
- [ ] **D3.** `spacing-effect` — `:OTHER` (no run recorded); determine whether it
      is a real run worth speccing.

## Phase E — hygiene (ongoing)

- [ ] **E1.** Probe coverage dashboard: pass / fail / ignored-by-reason plus the
      re-triage candidate list, so coverage is one command away.
- [ ] **E2.** Keep `GOTCHAS.md` current as fixes land.
- [x] Engine split into `rust_actr7/src/engine/*.rs` (commit `3b5c476`).
- [x] `cargo fmt` the crate (commit `8494640`).

---

## Explicitly out of scope (revisit only on request)

- The 91 command-output specs and the remaining command-only specs (need the
  command layer).
- The 344 non-timestamped unit-test refs.
